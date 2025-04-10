use std::error::Error;
use scraper::{Html, Selector};
use regex::Regex;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use anyhow::Result;

use crate::skysmart_api::SkysmartAPIClient;

pub fn remove_extra_newlines(text: &str) -> String {
    let re = Regex::new(r"\n+").unwrap();
    re.replace_all(text.trim(), "\n").to_string()
}

#[derive(Debug)]
pub struct TaskAnswer {
    pub question: String,
    pub answers: Vec<String>,
    pub task_number: usize,
}

pub struct SkyAnswers {
    task_hash: String,
}

impl SkyAnswers {
    pub fn new(task_hash: String) -> Self {
        Self { task_hash }
    }

    pub async fn get_answers(&self) -> Result<Vec<TaskAnswer>, Box<dyn Error + Send + Sync>> {
        let mut answers_list = Vec::new();
        let mut client = SkysmartAPIClient::new();

        // Ограничим количество обрабатываемых задач для предотвращения чрезмерного потребления памяти
        const MAX_TASKS: usize = 50;

        match client.get_room(&self.task_hash).await {
            Ok(tasks_uuids) => {
                // Ограничиваем количество задач, которые обрабатываем
                let tasks_to_process = tasks_uuids.iter().take(MAX_TASKS);
                
                for (idx, uuid) in tasks_to_process.enumerate() {
                    match client.get_task_html(uuid).await {
                        Ok(task_html) => {
                            let document = Html::parse_document(&task_html);
                            match self.get_task_answer(&document, idx + 1) {
                                Ok(task_answer) => answers_list.push(task_answer),
                                Err(e) => eprintln!("Error parsing task {}: {}", idx + 1, e),
                            }
                            // Очищаем HTML-документ из памяти после использования
                            drop(document);
                        },
                        Err(e) => {
                            eprintln!("Error fetching task HTML for UUID {}: {}", uuid, e);
                        }
                    }
                }
            },
            Err(e) => {
                eprintln!("Error in get_answers: {}", e);
            }
        }

        // Явно закрываем клиент для освобождения ресурсов
        client.close().await?;

        Ok(answers_list)
    }

    fn extract_task_full_question(&self, document: &Html) -> String {
        let text = document.root_element().text().collect::<String>();
        remove_extra_newlines(&text)
    }

    fn get_task_answer(&self, document: &Html, task_number: usize) -> Result<TaskAnswer> {
        let mut answers = Vec::new();

        if let Ok(selector) = Selector::parse("vim-test-item[correct='true']") {
            for element in document.select(&selector) {
                answers.push(element.text().collect::<String>());
            }
        }

        if let Ok(selector) = Selector::parse("vim-order-sentence-verify-item") {
            for element in document.select(&selector) {
                answers.push(element.text().collect::<String>());
            }
        }

        if let Ok(selector) = Selector::parse("vim-input-answers") {
            for input_answer in document.select(&selector) {
                if let Ok(item_selector) = Selector::parse("vim-input-item") {
                    if let Some(input_item) = input_answer.select(&item_selector).next() {
                        answers.push(input_item.text().collect::<String>());
                    }
                }
            }
        }

        if let Ok(selector) = Selector::parse("vim-select-item[correct='true']") {
            for element in document.select(&selector) {
                answers.push(element.text().collect::<String>());
            }
        }

        if let Ok(selector) = Selector::parse("vim-test-image-item[correct='true']") {
            for element in document.select(&selector) {
                answers.push(format!("{} - Correct", element.text().collect::<String>()));
            }
        }

        if let Ok(selector) = Selector::parse("math-input-answer") {
            for element in document.select(&selector) {
                answers.push(element.text().collect::<String>());
            }
        }

        if let Ok(selector) = Selector::parse("vim-dnd-text-drop") {
            for drop in document.select(&selector) {
                if let Some(drag_ids) = drop.value().attr("drag-ids") {
                    for drag_id in drag_ids.split(',') {
                        if let Ok(drag_selector) = Selector::parse(&format!("vim-dnd-text-drag[answer-id='{}']", drag_id)) {
                            if let Some(drag) = document.select(&drag_selector).next() {
                                answers.push(drag.text().collect::<String>());
                            }
                        }
                    }
                }
            }
        }

        if let Ok(selector) = Selector::parse("vim-groups-item") {
            for item in document.select(&selector) {
                if let Some(encoded_text) = item.value().attr("text") {
                    match BASE64.decode(encoded_text) {
                        Ok(decoded) => {
                            if let Ok(text) = String::from_utf8(decoded) {
                                answers.push(text);
                            }
                        },
                        Err(e) => eprintln!("Error decoding base64 text: {}", e),
                    }
                }
            }
        }

        let question = self.extract_task_full_question(document);

        Ok(TaskAnswer {
            question,
            answers,
            task_number,
        })
    }
}