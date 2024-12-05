#![feature(slice_pattern)]

mod renderer;
mod socket;

use core::slice::SlicePattern;
use std::{sync::mpsc, thread};

use dotenvy_macro::dotenv;
use escpos::{
    driver::UsbDriver,
    printer::Printer,
    printer_options::PrinterOptions,
    utils::{DebugMode, Protocol, ESC},
};
use renderer::print_message;
use serde::{Deserialize, Serialize};
use serenity::{
    all::{Context, EventHandler, GatewayIntents, Permissions},
    async_trait,
    model::channel::Message,
    prelude::{TypeMap, TypeMapKey},
    Client,
};
use socket::APISocket;
use twemoji_assets::png::PngTwemojiAsset;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum UnderlineMode {
    None,
    Single,
    Double,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum JustifyMode {
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum PrinterInstruction {
    Text(String),
    Image(String),
    Reverse(bool),
    Underline(UnderlineMode),
    Justify(JustifyMode),
    Strike(bool),
    Bold(bool),
    Italic(bool),
    PrintCut,
}

pub type PrinterMessage = Vec<PrinterInstruction>;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, context: Context, message: Message) {
        // if message.channel_id.get() != dotenv!("CHANNEL_ID").parse::<u64>().unwrap() {
        //     return;
        // }

        println!("lets check");
        if let Some(guild) = message.guild(&context.cache) {
            println!("in a guild");
            if let Some(role) = guild.role_by_name("@everyone") {
                println!("has @everyone");
                if !role.has_permission(Permissions::VIEW_CHANNEL) {
                    println!("can't see!");

                    return;
                }
            };
        };

        let type_map = context.data.as_ref().read().await;

        let Some(printer_channel_reference) = type_map.get::<PrinterChannel>() else {
            return;
        };

        let printer_channel = printer_channel_reference.clone();

        drop(type_map);

        let _ = print_message(&printer_channel, context, message).await;
    }
}

struct PrinterChannel;

impl TypeMapKey for PrinterChannel {
    type Value = mpsc::Sender<PrinterMessage>;
}

pub const CHARS_PER_LINE: u8 = 48;

#[tokio::main]
async fn main() {
    let (sender, receiver) = mpsc::channel::<PrinterMessage>();

    thread::spawn(move || {
        let driver = match UsbDriver::open(0x04B8, 0x0E20, None) {
            Ok(driver) => driver,
            Err(error) => {
                panic!("{:?}", error);
            }
        };

        let printer_options = PrinterOptions::new(None, Some(DebugMode::Hex), CHARS_PER_LINE);

        let mut printer = Printer::new(driver, Protocol::default(), Some(printer_options));

        match printer.init() {
            Ok(printer) => printer,
            Err(error) => {
                panic!("{:?}", error);
            }
        };

        //let emoji_regex = Regex::new(r"(\xC2\xA9|\xC2\AE|[\xE2\x80\x80-\xE3\x8C\x8C]|\xED\xA0\xBE[\x80\x80-\xBF\xBF]|\ud83d[\ud000-\udfff]|\ud83e[\ud000-\udfff])").unwrap();

        loop {
            let Ok(messages) = receiver.recv() else {
                continue;
            };

            println!("printer commands: {:?}", messages);

            let _ = printer.reverse(false);
            let _ = printer.underline(escpos::utils::UnderlineMode::None);
            let _ = printer.justify(escpos::utils::JustifyMode::LEFT);
            let _ = printer.double_strike(false);
            let _ = printer.bold(false);
            let _ = printer.custom(&[ESC, 0x35]);

            let mut last_command_was_print = false;

            for message in messages {
                last_command_was_print = false;

                match message {
                    PrinterInstruction::Text(text) => {
                        let message_bytes = text.as_bytes();
                        let message_len = message_bytes.len();
                        let mut i = 0_usize;

                        while i < message_len {
                            let curr_byte = message_bytes[i];
                            let mut curr_char = "".to_string();

                            if curr_byte < 0xC0 {
                                let Ok(full_char) = String::from_utf8(vec![message_bytes[i]])
                                else {
                                    continue;
                                };

                                curr_char = full_char;
                                i += 1;
                            } else if (0xC0..0xE0).contains(&curr_byte) {
                                let Ok(full_char) =
                                    String::from_utf8(vec![message_bytes[i], message_bytes[i + 1]])
                                else {
                                    continue;
                                };

                                curr_char = full_char;
                                i += 2;
                            } else if (0xE0..0xF0).contains(&curr_byte) {
                                let Ok(full_char) = String::from_utf8(vec![
                                    message_bytes[i],
                                    message_bytes[i + 1],
                                    message_bytes[i + 2],
                                ]) else {
                                    continue;
                                };

                                curr_char = full_char;
                                i += 3;
                            } else if curr_byte >= 0xF0 {
                                let Ok(full_char) = String::from_utf8(vec![
                                    message_bytes[i],
                                    message_bytes[i + 1],
                                    message_bytes[i + 2],
                                    message_bytes[i + 3],
                                ]) else {
                                    continue;
                                };

                                curr_char = full_char;
                                i += 4;
                            }

                            let Some(png_asset) = PngTwemojiAsset::from_emoji(&curr_char) else {
                                let _ = printer.write(&curr_char);
                                continue;
                            };

                            let png_data: &[u8] = png_asset;
                            let _ = printer.feed();
                            let _ = printer.bit_image_from_bytes(png_data);
                        }
                    }
                    PrinterInstruction::Image(url) => {
                        let Ok(image) = reqwest::blocking::get(url) else {
                            continue;
                        };

                        let Ok(image) = image.bytes() else {
                            continue;
                        };

                        let _ = printer.feed();
                        let _ = printer.bit_image_from_bytes(image.as_slice());
                    }
                    PrinterInstruction::Reverse(enabled) => {
                        let _ = printer.reverse(enabled);
                    }
                    PrinterInstruction::Underline(mode) => {
                        let _ = printer.underline(match mode {
                            UnderlineMode::None => escpos::utils::UnderlineMode::None,
                            UnderlineMode::Single => escpos::utils::UnderlineMode::Single,
                            UnderlineMode::Double => escpos::utils::UnderlineMode::Double,
                        });
                    }
                    PrinterInstruction::Justify(mode) => {
                        let _ = printer.justify(match mode {
                            JustifyMode::Left => escpos::utils::JustifyMode::LEFT,
                            JustifyMode::Center => escpos::utils::JustifyMode::CENTER,
                            JustifyMode::Right => escpos::utils::JustifyMode::RIGHT,
                        });
                    }
                    PrinterInstruction::Strike(enabled) => {
                        let _ = printer.double_strike(enabled);
                    }
                    PrinterInstruction::Bold(enabled) => {
                        let _ = printer.bold(enabled);
                    }
                    PrinterInstruction::Italic(enabled) => match enabled {
                        true => {
                            let _ = printer.custom(&[ESC, 0x34]);
                        }
                        false => {
                            let _ = printer.custom(&[ESC, 0x35]);
                        }
                    },
                    PrinterInstruction::PrintCut => {
                        let _ = printer.feed();
                        let _ = printer.partial_cut();
                        let _ = printer.print();
                        let _ = printer.debug();
                        last_command_was_print = true;
                    }
                };
            }

            if !last_command_was_print {
                let _ = printer.feed();
                let _ = printer.partial_cut();
                let _ = printer.print();
                let _ = printer.debug();
            }
        }
    });

    let api_job_sender = sender.clone();
    let (mut api_socket, api_receiver) = APISocket::create();

    let _ = thread::spawn(move || loop {
        let Ok(next_message) = api_receiver.recv() else {
            continue;
        };

        println!("from the API: {:?}", next_message);

        let _ = api_job_sender.send(next_message);
    });

    let _ = thread::spawn(move || api_socket.run());

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut printer_map = TypeMap::new();
    printer_map.insert::<PrinterChannel>(sender);

    let mut client = Client::builder(dotenv!("BOT_TOKEN"), intents)
        .event_handler(Handler)
        .type_map(printer_map)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
