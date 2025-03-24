# Rust Sky Answers Bot

## Описание

Telegram-бот на Rust для получения ответов на задания платформы Skysmart. Бот анализирует ссылку на задание и возвращает вопросы и ответы, если они доступны.

## Возможности

- Обработка ссылок на задания Skysmart
- Извлечение и отображение вопросов и ответов
- Удобный интерфейс взаимодействия через Telegram
- Автоматическое получение данных через API Skysmart

## Установка и запуск через Docker

Для запуска бота используйте следующие команды:

```bash
# Скачиваем образ из GitHub Container Registry
docker pull ghcr.io/myrsgral/rust-sky-answ:latest

# Запускаем контейнер с указанием токена бота Telegram
docker run -d --name rust-sky-answ -e TELOXIDE_TOKEN=ваш_токен_бота ghcr.io/myrsgral/rust-sky-answ:latest