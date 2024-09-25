use std::io::Cursor;

use discord_markdown::parser::{parse, Expression};
use dotenvy_macro::dotenv;
use genpdf::Document;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, context: Context, msg: Message) {
        if msg.channel_id.get() != dotenv!("CHANNEL_ID").parse::<u64>().unwrap() {
            return;
        }

        let font_family = genpdf::fonts::from_files("./fonts", "Inter_18pt", None)
            .expect("Failed to load font family");

        let mut doc = genpdf::Document::new(font_family);

        doc.push(genpdf::elements::Paragraph::new(format!(
            "{} @ {}",
            msg.author.name,
            msg.timestamp.format("%H:%M:%S %m-%d-%Y")
        )));
        let _ = render_vec_expr(&mut doc, &context, &parse(&msg.content));
        doc.push(genpdf::elements::Paragraph::new("Document content"));

        let mut pdf_vec = Vec::new();
        let pdf_buffer = Cursor::new(&mut pdf_vec);

        if doc.render(pdf_buffer).is_ok() {
            let printers = printers::get_printers();

            for printer in printers.clone() {
                println!("{:?}", printer);

                let _ = printer.print(pdf_vec.as_slice(), Some("Everything"));
            }
        };
    }
}

#[tokio::main]
async fn main() {
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(dotenv!("BOT_TOKEN"), intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

fn render_vec_expr(doc: &mut Document, context: &Context, parsed_content: &[Expression]) {
    let _ = parsed_content.iter().map(|e| render_expr(doc, context, e));
}

fn render_expr(doc: &mut Document, context: &Context, expr: &Expression) {
    match expr {
        Expression::Text(text) => doc.push(genpdf::elements::Paragraph::new(text.to_string())),
        Expression::CustomEmoji(emoji1, emoji2) => doc.push(genpdf::elements::Paragraph::new(
            format!("{}{}", emoji1, emoji2),
        )),
        Expression::User(user) => doc.push(genpdf::elements::Paragraph::new(user.to_string())),
        Expression::Role(role) => doc.push(genpdf::elements::Paragraph::new(role.to_string())),
        Expression::Channel(channel) => {
            doc.push(genpdf::elements::Paragraph::new(channel.to_string()))
        }
        Expression::Hyperlink(link1, link2) => doc.push(genpdf::elements::Paragraph::new(format!(
            "{}{}",
            link1, link2
        ))),
        Expression::MultilineCode(code) => {
            doc.push(genpdf::elements::Paragraph::new(code.to_string()))
        }
        Expression::InlineCode(code) => {
            doc.push(genpdf::elements::Paragraph::new(code.to_string()))
        }
        Expression::Blockquote(vec) => render_vec_expr(doc, context, vec),
        Expression::Spoiler(vec) => render_vec_expr(doc, context, vec),
        Expression::Underline(vec) => render_vec_expr(doc, context, vec),
        Expression::Strikethrough(vec) => render_vec_expr(doc, context, vec),
        Expression::Bold(vec) => render_vec_expr(doc, context, vec),
        Expression::Italics(vec) => render_vec_expr(doc, context, vec),
        Expression::Newline => doc.push(genpdf::elements::Paragraph::new("\n")),
    };
}
