use chrono::{DateTime, Datelike, Duration, Local, Timelike, Utc};
use comemo::Prehashed;
use discord_markdown::parser::{parse, Expression};
use fontdb::Database;
use serenity::all::{Context, Message};
use std::{fs, path::PathBuf, sync::OnceLock};

use time::Month;
use typst::{
    diag::FileResult,
    eval::Tracer,
    foundations::{Bytes, Datetime, Smart},
    syntax::{FileId, Source, VirtualPath},
    text::{Font, FontBook, FontInfo},
    Library, World,
};

pub struct FontSearcher {
    pub book: FontBook,
    pub fonts: Vec<FontSlot>,
}

pub struct FontSlot {
    path: PathBuf,
    index: u32,
    font: OnceLock<Option<Font>>,
}

impl FontSlot {
    pub fn get(&self) -> Option<Font> {
        self.font
            .get_or_init(|| {
                let data = fs::read(&self.path).ok()?.into();
                Font::new(data, self.index)
            })
            .clone()
    }
}

impl FontSearcher {
    pub fn new() -> Self {
        Self {
            book: FontBook::new(),
            fonts: vec![],
        }
    }

    pub fn search(&mut self) {
        let mut db = Database::new();

        db.load_system_fonts();

        for face in db.faces() {
            let path = match &face.source {
                fontdb::Source::File(path) | fontdb::Source::SharedFile(path, _) => path,
                fontdb::Source::Binary(_) => continue,
            };

            let info = db
                .with_face_data(face.id, FontInfo::new)
                .expect("database must contain this font");

            if let Some(info) = info {
                self.book.push(info);
                self.fonts.push(FontSlot {
                    path: path.clone(),
                    index: face.index,
                    font: OnceLock::new(),
                });
            }
        }

        self.add_embedded();
    }

    fn add_embedded(&mut self) {
        for data in typst_assets::fonts() {
            let buffer = typst::foundations::Bytes::from_static(data);
            for (i, font) in Font::iter(buffer).enumerate() {
                self.book.push(font.info().clone());
                self.fonts.push(FontSlot {
                    path: PathBuf::new(),
                    index: i as u32,
                    font: OnceLock::from(Some(font)),
                });
            }
        }
    }
}

struct CustomWorld {
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
    fonts: Vec<FontSlot>,
    main_file: Source,
}

impl World for CustomWorld {
    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &self.book
    }

    fn main(&self) -> Source {
        self.main_file.clone()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_file.id() {
            FileResult::Ok(self.main_file.clone())
        } else {
            FileResult::Err(typst::diag::FileError::NotFound(
                id.vpath().as_rooted_path().to_path_buf(),
            ))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        FileResult::Err(typst::diag::FileError::NotFound(
            id.vpath().as_rooted_path().to_path_buf(),
        ))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index)?.get()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        if let Some(offset) = offset {
            let utc: DateTime<Utc> = Utc::now() + Duration::hours(offset);

            Some(
                Datetime::construct(
                    Some(utc.year()),
                    Some(Month::try_from(utc.month() as u8).unwrap()),
                    Some(utc.day() as u8),
                    Some(utc.hour() as u8),
                    Some(utc.minute() as u8),
                    Some(utc.second() as u8),
                )
                .unwrap(),
            )
        } else {
            let local: DateTime<Local> = Local::now();

            Some(
                Datetime::construct(
                    Some(local.year()),
                    Some(Month::try_from(local.month() as u8).unwrap()),
                    Some(local.day() as u8),
                    Some(local.hour() as u8),
                    Some(local.minute() as u8),
                    Some(local.second() as u8),
                )
                .unwrap(),
            )
        }
    }
}

impl CustomWorld {
    fn new(file: String) -> CustomWorld {
        let mut fonts = FontSearcher::new();
        fonts.search();

        CustomWorld {
            library: Prehashed::new(Library::default()),
            book: Prehashed::new(fonts.book),
            fonts: fonts.fonts,
            main_file: Source::new(FileId::new_fake(VirtualPath::new("/")), file),
        }
    }
}

fn render_vec_expr(doc: &mut String, context: &Context, parsed_content: &[Expression]) {
    for expression in parsed_content {
        render_expr(doc, context, expression);
    }
}

fn render_expr(doc: &mut String, context: &Context, expr: &Expression) {
    match expr {
        Expression::Text(text) => *doc += text,
        Expression::CustomEmoji(_, emoji2) => {
            // *doc += &format!(
            //     "\n#box(\n  image(\"https://cdn.discordapp.com/emojis/{}\")\n)",
            //     emoji2
            // )
        }
        Expression::User(user) => *doc += user,
        Expression::Role(role) => *doc += role,
        Expression::Channel(channel) => *doc += channel,
        Expression::Hyperlink(link1, link2) => *doc += &format!(" #link(\"{}\")[{}]", link2, link1),
        Expression::MultilineCode(code) => {
            *doc += &format!(
                "\n#block(\n  fill: luma(230),\n  inset: 8pt,\n  radius: 4pt,\n  [{}],\n)\n",
                code
            )
        }
        Expression::InlineCode(code) => *doc += &format!("`{}`", code),
        Expression::Blockquote(vec) => {
            *doc += " \\\n\"";
            render_vec_expr(doc, context, vec);
            *doc += "\" \\";
        }
        Expression::Spoiler(vec) => {
            *doc += "#highlight[";
            render_vec_expr(doc, context, vec);
            *doc += "]";
        }
        Expression::Underline(vec) => {
            *doc += "#underline[";
            render_vec_expr(doc, context, vec);
            *doc += "]";
        }
        Expression::Strikethrough(vec) => {
            *doc += "#strike[";
            render_vec_expr(doc, context, vec);
            *doc += "]";
        }
        Expression::Bold(vec) => {
            *doc += "*";
            render_vec_expr(doc, context, vec);
            *doc += "*";
        }
        Expression::Italics(vec) => {
            *doc += "_";
            render_vec_expr(doc, context, vec);
            *doc += "_";
        }
        Expression::Newline => *doc += " \\",
    };
}

pub enum RenderError {
    CompileFailed,
}

pub fn render(context: Context, msg: Message) -> Result<Vec<u8>, RenderError> {
    let mut render_result = "".to_string();

    render_vec_expr(&mut render_result, &context, &parse(&msg.content));

    println!("{}", render_result);

    let world = CustomWorld::new(render_result);
    let mut tracer = Tracer::new();

    let result = typst::compile(&world, &mut tracer);

    let Ok(document) = result else {
        println!("Compilation Failed: {:?}", result);

        return Err(RenderError::CompileFailed);
    };

    let local: DateTime<Local> = Local::now();

    Ok(typst_pdf::pdf(
        &document,
        Smart::Auto,
        Some(
            Datetime::construct(
                Some(local.year()),
                Some(Month::try_from(local.month() as u8).unwrap()),
                Some(local.day() as u8),
                Some(local.hour() as u8),
                Some(local.minute() as u8),
                Some(local.second() as u8),
            )
            .unwrap(),
        ),
    ))
}
