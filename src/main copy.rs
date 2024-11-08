use std::{
    env,
    fs::File,
    io::{Cursor, Write},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Datelike, Duration, Local, Timelike, Utc};
use comemo::Prehashed;
use discord_markdown::parser::{parse, Expression};
use dotenvy_macro::dotenv;
use serenity::{async_trait, model::channel::Message, prelude::*};

use time::Month;
use tokio::fs;
use typst::{
    diag::FileResult,
    eval::Tracer,
    foundations::{Bytes, Datetime, Smart},
    syntax::{FileId, Source, VirtualPath},
    text::{Font, FontBook},
    Library, World,
};

struct CustomWorld {
    library: Prehashed<Library>,
    book: Prehashed<FontBook>,
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
        println!("Requested font {index}");

        None
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
        CustomWorld {
            library: Prehashed::new(Library::default()),
            book: Prehashed::new(FontBook::new()),
            main_file: Source::new(FileId::new_fake(VirtualPath::new("/")), file),
        }
    }
}

// struct Handler;

// #[async_trait]
// impl EventHandler for Handler {
//     async fn message(&self, context: Context, msg: Message) {
//         if msg.channel_id.get() != dotenv!("CHANNEL_ID").parse::<u64>().unwrap() {
//             return;
//         }

//         println!("{:?}", parse(&msg.content));
//         // let _ = render_vec_expr(&mut doc, &context, &parse(&msg.content));

//         let world = CustomWorld::new("= hello world\nhello!".to_string());
//         let mut tracer = Tracer::new();

//         let result = typst::compile(&world, &mut tracer);

//         if let Ok(document) = result {
//             println!("{:?}", document);

//             let pdf_vec = typst_pdf::pdf(&document, Smart::Auto, None);

//             println!("{:?}", pdf_vec);

//             let time = SystemTime::now()
//                 .duration_since(UNIX_EPOCH)
//                 .unwrap()
//                 .subsec_nanos();

//             let tmp_file_path = env::current_dir()
//                 .unwrap()
//                 .join(format!("target/{}.pdf", time)); //env::temp_dir().join(format!("{}.pdf", time));

//             let save = std::fs::write(tmp_file_path.clone(), pdf_vec.as_slice());

//             if save.is_err() {
//                 return;
//             }

//             // let printers = printers::get_printers();

//             // for printer in printers.clone() {
//             //     println!("{:?}", printer);

//             //     let _ = printer.print_file(
//             //         tmp_file_path.to_str().unwrap(),
//             //         Some("Awestruck Message!!!"),
//             //     );
//             // }
//         }
//     }
// }

#[tokio::main]
async fn main() {
    // let intents = GatewayIntents::GUILD_MESSAGES
    //     | GatewayIntents::DIRECT_MESSAGES
    //     | GatewayIntents::MESSAGE_CONTENT;

    // let mut client = Client::builder(dotenv!("BOT_TOKEN"), intents)
    //     .event_handler(Handler)
    //     .await
    //     .expect("Err creating client");

    // if let Err(why) = client.start().await {
    //     println!("Client error: {why:?}");
    // }

    let world = CustomWorld::new("= hello world\nhello!".to_string());
    let mut tracer = Tracer::new();

    let result = typst::compile(&world, &mut tracer);

    println!("{:?}", tracer.clone().delayed());
    println!("{:?}", tracer.clone().warnings());
    println!("{:?}", tracer.clone().values());

    if let Ok(document) = result {
        println!("{:?}", document);

        let local: DateTime<Local> = Local::now();

        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();

        let tmp_file_path = env::current_dir()
            .unwrap()
            .join(format!("target/{}.pdf", time)); //env::temp_dir().join(format!("{}.pdf", time));

        let pdf_vec = typst_pdf::pdf(
            &document,
            Smart::Custom(&format!("{}", time)),
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
        );

        println!("{:?}", pdf_vec);

        let save = std::fs::write(tmp_file_path.clone(), pdf_vec.as_slice());

        if save.is_err() {
            return;
        }

        // let printers = printers::get_printers();

        // for printer in printers.clone() {
        //     println!("{:?}", printer);

        //     let _ = printer.print_file(
        //         tmp_file_path.to_str().unwrap(),
        //         Some("Awestruck Message!!!"),
        //     );
        // }
    }
}

// fn render_vec_expr(doc: &mut String, context: &Context, parsed_content: &[Expression]) {
//     for expression in parsed_content {
//         render_expr(doc, context, expression);
//     }
// }

// fn render_expr(doc: &mut String, context: &Context, expr: &Expression) {
//     // match expr {
//     //     // Expression::Text(text) => doc.push(genpdf::elements::Paragraph::new(text.to_string())),
//     //     // Expression::CustomEmoji(emoji1, emoji2) => doc.push(genpdf::elements::Paragraph::new(
//     //     //     format!("{}{}", emoji1, emoji2),
//     //     // )),
//     //     // Expression::User(user) => doc.push(genpdf::elements::Paragraph::new(user.to_string())),
//     //     // Expression::Role(role) => doc.push(genpdf::elements::Paragraph::new(role.to_string())),
//     //     // Expression::Channel(channel) => {
//     //     //     doc.push(genpdf::elements::Paragraph::new(channel.to_string()))
//     //     // }
//     //     // Expression::Hyperlink(link1, link2) => doc.push(genpdf::elements::Paragraph::new(format!(
//     //     //     "{}{}",
//     //     //     link1, link2
//     //     // ))),
//     //     // Expression::MultilineCode(code) => {
//     //     //     doc.push(genpdf::elements::Paragraph::new(code.to_string()))
//     //     // }
//     //     // Expression::InlineCode(code) => {
//     //     //     doc.push(genpdf::elements::Paragraph::new(code.to_string()))
//     //     // }
//     //     // Expression::Blockquote(vec) => render_vec_expr(doc, context, vec),
//     //     // Expression::Spoiler(vec) => render_vec_expr(doc, context, vec),
//     //     // Expression::Underline(vec) => render_vec_expr(doc, context, vec),
//     //     // Expression::Strikethrough(vec) => render_vec_expr(doc, context, vec),
//     //     // Expression::Bold(vec) => render_vec_expr(doc, context, vec),
//     //     // Expression::Italics(vec) => render_vec_expr(doc, context, vec),
//     //     // Expression::Newline => doc.push(genpdf::elements::Break::new(1)),
//     // };
// }
