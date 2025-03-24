mod skysmart_api;
mod api_constants;
mod answer_module;

use std::error::Error as StdError;
use dotenv::dotenv;
use std::env;

use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::dispatching::Dispatcher;
use teloxide::dispatching::HandlerExt;
use teloxide::types::Message;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Список доступных команд:")]
enum Command {
    #[command(description = "Запустить бота")]
    Start,
    #[command(description = "Помощь")]
    Help,
}


#[derive(Debug)]
pub struct Solution {
    pub task_number: usize,
    pub question: String,
    pub answers: Vec<String>,
}

struct SkyAnswers {
    task_hash: String,
}

impl SkyAnswers {
    pub fn new(task_hash: String) -> Self {
        Self { task_hash }
    }

    pub async fn get_answers(&self) -> Result<Vec<Solution>, Box<dyn StdError + Send + Sync + 'static>> {
        Ok(vec![])
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError + Send + Sync>> {
    pretty_env_logger::init();
    dotenv().ok();
    let token = env::var("TELOXIDE_TOKEN")
        .expect("Не найдена переменная окружения TELOXIDE_TOKEN. Убедитесь, что она прописана в .env");

    let bot = Bot::new(token);


    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<Command>()
                .endpoint(commands_handler),
        )
        .branch(
            dptree::entry()
                .filter(|msg: Message| msg.text().is_some())
                .endpoint(|bot, msg| async move {
                    message_handler(bot, msg).await
                }),
        );

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn commands_handler(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Start => {
            bot.send_message(msg.chat.id, "Привет! Я бот, который поможет тебе пройти тест. Просто отправь мне ссылку на тест, и я постараюсь дать тебе ответы. Удачи! 😉").await?;
        }
        Command::Help => {
            bot.send_message(msg.chat.id, "Привет! Я бот, который поможет тебе пройти тест. Просто отправь мне ссылку на тест, и я постараюсь дать тебе ответы. Удачи! 😉").await?;
        }
    }
    Ok(())
}


async fn message_handler(bot: Bot, msg: Message) -> ResponseResult<()> {
    if let Some(text) = msg.text() {
        log::info!("Received message: {}", text);

        let trimmed = text.trim();
        if trimmed.is_empty() {
            bot.send_message(msg.chat.id, "⚠️ Пожалуйста, отправьте корректную ссылку на задание.").await?;
            return Ok(());
        }

        let mut task_hash = trimmed.to_string();

        task_hash = task_hash
            .replace("https://edu.skysmart.ru/student/", "")
            .replace("http://edu.skysmart.ru/student/", "")
            .replace("edu.skysmart.ru/student/", "");
        if task_hash.is_empty() || task_hash.len() < 5 {
            bot.send_message(msg.chat.id, "⚠️ Неверный формат ссылки. Отправьте полную ссылку на задание.").await?;
            return Ok(());
        }

        let processing_msg = match bot.send_message(msg.chat.id, "🔍 Ищу ответы, подождите...").await {
            Ok(msg) => msg,
            Err(e) => {
                log::error!("Failed to send processing message: {}", e);
                return Ok(());
            }
        };

        let answers_module = answer_module::SkyAnswers::new(task_hash.clone());

        let result = match answers_module.get_answers().await {
            Ok(answers) => {
                if answers.is_empty() {
                    match bot.send_message(msg.chat.id, "❌ Не удалось найти ответы для этого задания.").await {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            log::error!("Failed to send empty answers message: {}", e);
                            Err(e)
                        }
                    }
                } else {
                    for solution in answers {
                        let mut task_message = format!("📝 Задание #{}\n━━━━━━━━━━━━━━━━━━━\n\n", solution.task_number);
                        task_message.push_str(&format!("{}\n\n", solution.question));
                        task_message.push_str("🔍 ОТВЕТЫ:\n\n");
                        if solution.answers.len() > 1 {
                            for (i, answer) in solution.answers.iter().enumerate() {
                                task_message.push_str(&format!("✅ Ответ {}: {}\n", i+1, answer));
                            }
                        } else if let Some(answer) = solution.answers.first() {
                            task_message.push_str(&format!("✅ Ответ: {}\n", answer));
                        }
                        task_message.push_str("\n━━━━━━━━━━━━━━━━━━━");

                        if let Err(e) = bot.send_message(msg.chat.id, task_message).await {
                            log::error!("Failed to send answer: {}", e);
                            return Err(e);
                        }
                    }
                    if let Err(e) = bot.send_message(msg.chat.id, "✅ Выдача ответов завершена!").await {
                        log::error!("Failed to send completion message: {}", e);
                        return Err(e);
                    }
                    Ok(())
                }
            },
            Err(e) => {
                log::error!("Error getting answers for hash {}: {}", task_hash, e);
                match bot.send_message(
                    msg.chat.id,
                    format!("❌ Ошибка при получении ответов: {}", e)
                ).await {
                    Ok(_) => Ok(()),
                    Err(send_err) => {
                        log::error!("Failed to send error message: {}", send_err);
                        Err(send_err)
                    }
                }
            }
        };

        if let Err(e) = bot.delete_message(msg.chat.id, processing_msg.id).await {
            log::warn!("Failed to delete processing message: {}", e);
        }
        return result;
    }

    Ok(())
}