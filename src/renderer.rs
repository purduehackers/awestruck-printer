use discord_markdown::parser::{parse, Expression};
use regex::Regex;
use serenity::all::{
    ArgumentConvert, Channel, Context, Message, Role, User,
};
use std::sync::mpsc;

use crate::{PrinterInstruction, PrinterMessage, UnderlineMode, CHARS_PER_LINE};

async fn render_vec_expr(
    printer_commands: &mut PrinterMessage,
    context: &Context,
    message: &Message,
    parsed_content: &[Expression<'_>],
) {
    Box::pin(async move {
        for expression in parsed_content {
            render_expr(printer_commands, context, message, expression).await;
        }
    })
    .await;
}

async fn render_expr(
    printer_commands: &mut PrinterMessage,
    context: &Context,
    message: &Message,
    expr: &Expression<'_>,
) {
    match expr {
        Expression::Text(text) => {
            //for c in text.chars() {
            //    printer_commands.push(PrinterInstruction::Text((c).to_string() + "MEOW"));
            //}

            //for val in text.ascii_chars().map(|ch| ch.unwrap_or("?")) {
            //    printer_commands.push(PrinterInstruction::Text(((*val).to_owned() + " ").to_owned()));
            //}
            printer_commands.push(PrinterInstruction::Text(((*text).to_owned()).to_owned()));
        }
        Expression::CustomEmoji(_, emoji2) => {
            printer_commands.push(PrinterInstruction::Image(
                format!("https://cdn.discordapp.com/emojis/{}?size=64", emoji2).to_owned(),
            ));
        }
        //Expression::CustomEmoji(_, emoji2) => {
        //    printer_commands.push(PrinterInstruction::Image(
        //        format!("https://cdn.discordapp.com/emojis/{}?size=64", emoji2).to_owned(),
        //    ));
        //}
        Expression::User(user) => {
            printer_commands.push(PrinterInstruction::Reverse(true));

            let Ok(user) =
                User::convert(context, message.guild_id, Some(message.channel_id), user).await
            else {
                printer_commands.push(PrinterInstruction::Text("@Unknown User".to_owned()));

                printer_commands.push(PrinterInstruction::Reverse(false));
                return;
            };

            let Some(guild_id) = message.guild_id else {
                printer_commands.push(PrinterInstruction::Text(
                    format!("@{}", user.name).to_owned(),
                ));

                printer_commands.push(PrinterInstruction::Reverse(false));
                return;
            };

            let name = match user.nick_in(context, guild_id).await {
                Some(name) => name,
                None => user.name,
            };

            printer_commands.push(PrinterInstruction::Text(format!("@{}", name).to_owned()));

            printer_commands.push(PrinterInstruction::Reverse(false));
        }
        Expression::Role(role) => {
            printer_commands.push(PrinterInstruction::Reverse(true));

            let Ok(role) =
                Role::convert(context, message.guild_id, Some(message.channel_id), role).await
            else {
                printer_commands.push(PrinterInstruction::Text("@Unknown Role".to_owned()));

                printer_commands.push(PrinterInstruction::Reverse(false));
                return;
            };

            printer_commands.push(PrinterInstruction::Text(
                format!("@{}", role.name).to_owned(),
            ));

            printer_commands.push(PrinterInstruction::Reverse(false));
        }
        Expression::Channel(channel) => {
            printer_commands.push(PrinterInstruction::Reverse(true));

            let Ok(channel) =
                Channel::convert(context, message.guild_id, Some(message.channel_id), channel)
                    .await
            else {
                printer_commands.push(PrinterInstruction::Text("#Unknown Channel".to_owned()));

                printer_commands.push(PrinterInstruction::Reverse(false));
                return;
            };

            let Some(channel) = channel.clone().guild() else {
                let Some(channel) = channel.private() else {
                    printer_commands.push(PrinterInstruction::Text("#Unknown Channel".to_owned()));

                    printer_commands.push(PrinterInstruction::Reverse(false));
                    return;
                };

                printer_commands.push(PrinterInstruction::Text(
                    format!("#{}", channel.name()).to_owned(),
                ));

                printer_commands.push(PrinterInstruction::Reverse(false));
                return;
            };

            printer_commands.push(PrinterInstruction::Text(
                format!("#{}", channel.name).to_owned(),
            ));

            printer_commands.push(PrinterInstruction::Reverse(false));
        }
        Expression::Hyperlink(link1, _) => {
            let Some(_caps) = Regex::new(r"\.(jpg|jpeg|png|webp|gif)")
                .unwrap()
                .captures(link1)
            else {
                printer_commands.push(PrinterInstruction::Underline(UnderlineMode::Single));
                printer_commands.push(PrinterInstruction::Text(link1.to_string()));
                printer_commands.push(PrinterInstruction::Underline(UnderlineMode::None));
                return;
            };

            printer_commands.push(PrinterInstruction::Image(link1.to_string().to_owned()));
        }
        Expression::MultilineCode(code) => {
            printer_commands.push(PrinterInstruction::Reverse(true));
            let mut text_elements: Vec<String> = Vec::new();

            text_elements.push(format!("\n{}", " ".repeat(CHARS_PER_LINE.into())));

            let mut character_index = 0;

            for line in code.split("\n") {
                text_elements.push("\n  ".to_string());

                let line_length = line.len();
                let mut characters_printed = 0;

                for char in line.chars() {
                    text_elements.push(format!("{}", char));
                    character_index += 1;
                    characters_printed += 1;

                    if character_index >= CHARS_PER_LINE - 4 && characters_printed < line_length {
                        character_index = 0;

                        text_elements.push("  \n".to_string());
                        text_elements.push("  ".to_string());
                    }
                }

                text_elements.push("  ".to_string());
            }

            text_elements.push(" ".repeat((CHARS_PER_LINE - character_index - 4).into()));

            text_elements.push(format!("\n{}\n", " ".repeat(CHARS_PER_LINE.into())));

            printer_commands.push(PrinterInstruction::Text(text_elements.join("")));

            printer_commands.push(PrinterInstruction::Reverse(false));
        }
        Expression::InlineCode(code) => {
            printer_commands.push(PrinterInstruction::Reverse(true));
            printer_commands.push(PrinterInstruction::Text((*code).to_owned()));
            printer_commands.push(PrinterInstruction::Reverse(false));
        }
        Expression::Blockquote(vec) => {
            printer_commands.push(PrinterInstruction::Text("\"".to_owned()));
            render_vec_expr(printer_commands, context, message, vec).await;
            printer_commands.push(PrinterInstruction::Text("\"\n".to_owned()));
        }
        Expression::Spoiler(_) => {
            printer_commands.push(PrinterInstruction::Reverse(true));
            printer_commands.push(PrinterInstruction::Text(" SPOILER ".to_owned()));
            printer_commands.push(PrinterInstruction::Reverse(false));
        }
        Expression::Underline(vec) => {
            printer_commands.push(PrinterInstruction::Underline(UnderlineMode::Single));
            render_vec_expr(printer_commands, context, message, vec).await;
            printer_commands.push(PrinterInstruction::Underline(UnderlineMode::None));
        }
        Expression::Strikethrough(vec) => {
            printer_commands.push(PrinterInstruction::Strike(true));
            render_vec_expr(printer_commands, context, message, vec).await;
            printer_commands.push(PrinterInstruction::Strike(false));
        }
        Expression::Bold(vec) => {
            printer_commands.push(PrinterInstruction::Bold(true));
            render_vec_expr(printer_commands, context, message, vec).await;
            printer_commands.push(PrinterInstruction::Bold(false));
        }
        Expression::Italics(vec) => {
            printer_commands.push(PrinterInstruction::Italic(true));
            render_vec_expr(printer_commands, context, message, vec).await;
            printer_commands.push(PrinterInstruction::Italic(false));
        }
        Expression::Newline => {
            printer_commands.push(PrinterInstruction::Text("\n".to_owned()));
        }
    };
}

pub async fn print_message(
    printer: &mpsc::Sender<PrinterMessage>,
    context: Context,
    message: Message,
) {
    println!("{:?}", message);

    let context = &context;
    let message = &message;

    let author_name = if let Some(guild_id) = message.guild_id {
        match message.author.nick_in(context, guild_id).await {
            Some(name) => name,
            None => message.author.name.clone(),
        }
    } else {
        message.author.name.clone()
    };

    let channel_name = if let Ok(channel) = message.channel(context).await {
        if let Some(channel) = channel.guild() {
            format!("#{}", channel.name).to_string()
        } else {
            "#Unknown Channel".to_string()
        }
    } else {
        "Direct Messages".to_string()
    };

    let mut printer_commands = PrinterMessage::new();

    printer_commands.push(PrinterInstruction::Reverse(true));
    printer_commands.push(PrinterInstruction::Text(
        format!("@{}", author_name).to_owned(),
    ));
    printer_commands.push(PrinterInstruction::Reverse(false));

    printer_commands.push(PrinterInstruction::Text(" in ".to_owned()));

    printer_commands.push(PrinterInstruction::Reverse(true));
    printer_commands.push(PrinterInstruction::Text(channel_name.to_owned()));
    printer_commands.push(PrinterInstruction::Reverse(false));

    printer_commands.push(PrinterInstruction::Text("\n\n".to_owned()));

    render_vec_expr(
        &mut printer_commands,
        context,
        message,
        &parse(&message.content),
    )
    .await;

    let attachments = &message.attachments;
    let stickers = &message.sticker_items;

    for sticker in stickers {
        if let Some(sticker_url) = &sticker.image_url() {
            printer_commands.push(PrinterInstruction::Image(
                sticker_url.to_owned(),
            ));
        }
    }

    if let Ok(file_regex) = Regex::new(r"image/") {
        for attachment in attachments {
            if let Some(attachment_type) = &attachment.content_type {
                let Some(_) = file_regex.captures(attachment_type)
                else {
                    printer_commands.push(PrinterInstruction::Underline(UnderlineMode::Single));
                    printer_commands.push(PrinterInstruction::Text(format!("\n\nFile: {}", attachment.filename.clone())));
                    printer_commands.push(PrinterInstruction::Underline(UnderlineMode::None));
                    continue;
                };
    
                printer_commands.push(PrinterInstruction::Underline(UnderlineMode::Single));
                printer_commands.push(PrinterInstruction::Text(format!("\n\nFile: {}", attachment.filename.clone())));
                printer_commands.push(PrinterInstruction::Underline(UnderlineMode::None));
                printer_commands.push(PrinterInstruction::Image(
                    attachment.proxy_url.to_string().to_owned(),
                ));
            }
        }
    }

    printer_commands.push(PrinterInstruction::PrintCut);

    let _ = printer.send(printer_commands);
}
