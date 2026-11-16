# TeamTalk XML Monitor Bot

Telegram-бот для мониторинга изменений XML-конфигурации сервера TeamTalk 5. При любом изменении конфигурации отправляет уведомление с описанием изменений и актуальный XML-файл.

## Возможности

- Мониторинг учётных записей (создание, удаление, изменение прав, паролей, типа пользователя)
- Мониторинг каналов (создание, удаление, изменение настроек)
- Мониторинг блокировок (баны по IP и имени пользователя)
- Мониторинг настроек сервера, файлового хранилища и логирования
- Уведомления на русском языке с описанием конкретных изменений
- Автоматическая отправка актуального XML-файла

## Требования

- Rust 1.70+ (основная реализация) или Python 3.8+ (альтернативная реализация)
- Сервер TeamTalk 5 (Linux)
- Telegram-бот (токен от @BotFather)

## Установка (Rust)

### 1. Клонирование репозитория

```bash
git clone https://github.com/<your-github-username>/teamtalk-xml-bot.git
cd teamtalk-xml-bot
```

### 2. Настройка

Скопируйте `.env.example` в `.env` и укажите свои значения:

```bash
cp .env.example .env
```

Отредактируйте `.env`:

```ini
BOT_TOKEN=ваш_токен_бота
CHAT_ID=ваш_chat_id
XML_PATH=/home/tt/config.xml
CHECK_INTERVAL=5
```

### 3. Сборка

```bash
cargo build --release
```

### 4. Запуск

```bash
./target/release/teamtalk-xml-bot
```

### 5. Установка как systemd-сервис

Создайте `/etc/systemd/system/xml-monitor.service`:

```ini
[Unit]
Description=TeamTalk XML Monitor Bot
After=network.target

[Service]
Type=simple
WorkingDirectory=/path/to/teamtalk-xml-bot
ExecStart=/path/to/teamtalk-xml-bot/target/release/teamtalk-xml-bot
Restart=always
RestartSec=5
EnvironmentFile=/path/to/teamtalk-xml-bot/.env

[Install]
WantedBy=multi-user.target
```

Запуск:

```bash
sudo systemctl daemon-reload
sudo systemctl enable xml-monitor
sudo systemctl start xml-monitor
```

## Установка (Python)

```bash
pip3 install python-telegram-bot
```

Отредактируйте `xml-monitor.service`, укажите токен, ID чата и путь к XML-файлу:

```ini
[Service]
Environment="BOT_TOKEN=ваш_токен_бота"
Environment="CHAT_ID=ваш_chat_id"
Environment="XML_PATH=/home/tt/config.xml"
ExecStart=/usr/bin/python3 /home/tt/xml_bot.py
```

Затем:

```bash
sudo cp xml-monitor.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable xml-monitor
sudo systemctl start xml-monitor
```

## Переменные окружения

| Переменная | Описание | По умолчанию |
|------------|----------|--------------|
| `BOT_TOKEN` | Токен Telegram-бота | — |
| `CHAT_ID` | ID чата для уведомлений | — |
| `XML_PATH` | Путь к XML-файлу TeamTalk | `/home/tt/config.xml` |
| `CHECK_INTERVAL` | Интервал проверки (секунды) | `5` |

## Как это работает

1. Бот каждые 5 секунд проверяет хеш XML-файла (SHA-256)
2. При изменении сравнивает текущую версию с предыдущей
3. Формирует человекочитаемое описание изменений на русском языке
4. Отправляет уведомление и актуальный XML-файл в Telegram

## Примеры уведомлений

```
Изменения на сервере в 12:00:00:

Учётная запись <имя>: назначена администратором
```

```
Изменения на сервере в 12:00:00:

Настройки сервера:
  • Макс. пользователей: <old> → <new>
```

## Структура проекта

```
teamtalk-xml-bot/
├── Cargo.toml           # Зависимости Rust
├── .env.example         # Пример конфигурации
├── src/
│   └── main.rs          # Основной код (Rust)
├── xml_bot.py           # Альтернативная реализация (Python)
├── xml-monitor.service  # systemd unit файл
└── README.md            # Документация
```

## Лицензия

Copyright (c) 2026. Все права защищены.

Запрещено без письменного разрешения правообладателя:

- Копировать, изменять, модифицировать или создавать производные произведения на основе этого кода
- Распространять, публиковать или передавать код третьим лицам
- Использовать код в коммерческих или личных целях

Разрешается только ознакомление с исходным кодом. Любое использование, изменение или распространение допускается исключительно с письменного согласия автора.