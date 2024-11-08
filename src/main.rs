mod renderer;

use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};

use dotenvy_macro::dotenv;
use renderer::render;
use serenity::{async_trait, model::channel::Message, prelude::*};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, context: Context, msg: Message) {
        if msg.channel_id.get() != dotenv!("CHANNEL_ID").parse::<u64>().unwrap() {
            return;
        }

        let render_result = render(context, msg);

        let Ok(render_data) = render_result else {
            return;
        };

        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();

        let tmp_file_path = env::current_dir()
            .unwrap()
            .join(format!("target/{}.pdf", time));

        let write_result = std::fs::write(tmp_file_path.clone(), render_data.as_slice());

        if write_result.is_err() {
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
