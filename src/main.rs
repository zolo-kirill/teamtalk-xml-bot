use chrono::Local;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    bot_token: String,
    chat_id: String,
    xml_path: String,
    check_interval: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserAccount {
    #[serde(rename = "username")]
    username: String,
    #[serde(rename = "user-type", default)]
    user_type: Option<String>,
    #[serde(rename = "user-rights", default)]
    user_rights: Option<String>,
    #[serde(rename = "password", default)]
    password: Option<String>,
    #[serde(rename = "note", default)]
    note: Option<String>,
    #[serde(rename = "init-channel", default)]
    init_channel: Option<String>,
    #[serde(rename = "userdata", default)]
    userdata: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Channel {
    #[serde(rename = "channel-id")]
    channel_id: String,
    #[serde(rename = "name", default)]
    name: Option<String>,
    #[serde(rename = "password", default)]
    password: Option<String>,
    #[serde(rename = "topic", default)]
    topic: Option<String>,
    #[serde(rename = "diskquota", default)]
    diskquota: Option<String>,
    #[serde(rename = "max-users", default)]
    max_users: Option<String>,
    #[serde(rename = "channel-type", default)]
    channel_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TeamTalkConfig {
    #[serde(rename = "general")]
    general: Option<General>,
    #[serde(rename = "users")]
    users: Option<Users>,
    #[serde(rename = "permanent-channels")]
    permanent_channels: Option<PermanentChannels>,
}

#[derive(Debug, Serialize, Deserialize)]
struct General {
    #[serde(rename = "upnp", default)]
    upnp: Option<String>,
    #[serde(rename = "server-name", default)]
    server_name: Option<String>,
    #[serde(rename = "motd", default)]
    motd: Option<String>,
    #[serde(rename = "auto-save", default)]
    auto_save: Option<String>,
    #[serde(rename = "max-users", default)]
    max_users: Option<String>,
    #[serde(rename = "tcpport", default)]
    tcpport: Option<String>,
    #[serde(rename = "udpport", default)]
    udpport: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Users {
    #[serde(rename = "user", default)]
    users: Vec<UserAccount>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PermanentChannels {
    #[serde(rename = "channel", default)]
    channels: Vec<Channel>,
}

const USER_TYPE_MAP: &[(&str, &str, &str)] = &[
    ("0", "1", "учётная запись активирована"),
    ("1", "0", "учётная запись отключена"),
    ("1", "2", "назначена администратором"),
    ("2", "1", "сняты права администратора"),
    ("0", "2", "назначена администратором"),
    ("2", "0", "учётная запись отключена"),
];

const GENERAL_LABELS: &[(&str, &str)] = &[
    ("upnp", "UPnP"),
    ("server-name", "Название сервера"),
    ("motd", "Сообщение дня"),
    ("auto-save", "Автосохранение"),
    ("max-users", "Макс. пользователей"),
    ("tcpport", "TCP порт"),
    ("udpport", "UDP порт"),
];

const CHANNEL_LABELS: &[(&str, &str)] = &[
    ("name", "Название"),
    ("password", "Пароль"),
    ("topic", "Тема"),
    ("diskquota", "Дисковая квота"),
    ("max-users", "Макс. пользователей"),
    ("channel-type", "Тип канала"),
];

const USER_RIGHTS_MAP: &[(u32, &str)] = &[
    (0x00000001, "множественный вход"),
    (0x00000002, "просмотр всех пользователей"),
    (0x00000004, "создание временных каналов"),
    (0x00000008, "изменение каналов"),
    (0x00000010, "широковещательные сообщения"),
    (0x00000020, "исключение пользователей"),
    (0x00000040, "блокировка пользователей"),
    (0x00000080, "перемещение пользователей"),
    (0x00000100, "включение операторского режима"),
    (0x00000200, "загрузка файлов"),
    (0x00000400, "скачивание файлов"),
    (0x00000800, "обновление настроек сервера"),
    (0x00001000, "передача голоса"),
    (0x00002000, "передача видео"),
    (0x00004000, "передача рабочего стола"),
    (0x00008000, "удалённое управление рабочим столом"),
    (0x00010000, "передача аудиофайлов"),
    (0x00020000, "передача видеофайлов"),
    (0x00040000, "фиксация никнейма"),
    (0x00080000, "фиксация статуса"),
    (0x00100000, "запись голоса"),
    (0x00200000, "просмотр скрытых каналов"),
    (0x00400000, "личные сообщения"),
    (0x00800000, "сообщения в канале"),
];

fn parse_user_rights(value: &str) -> Vec<&'static str> {
    let rights_int: u32 = value.parse().unwrap_or(0);
    USER_RIGHTS_MAP
        .iter()
        .filter(|(bit, _)| rights_int & bit != 0)
        .map(|(_, name)| *name)
        .collect()
}

fn describe_changes(old_path: &str, new_path: &str) -> Vec<String> {
    let old_content = match fs::read_to_string(old_path) {
        Ok(c) => c,
        Err(_) => return vec!["Ошибка чтения предыдущей версии".to_string()],
    };
    let new_content = match fs::read_to_string(new_path) {
        Ok(c) => c,
        Err(_) => return vec!["Ошибка чтения текущей версии".to_string()],
    };

    let old_config: TeamTalkConfig = match quick_xml::de::from_str(&old_content) {
        Ok(c) => c,
        Err(_) => return vec!["Ошибка парсинга XML".to_string()],
    };
    let new_config: TeamTalkConfig = match quick_xml::de::from_str(&new_content) {
        Ok(c) => c,
        Err(_) => return vec!["Ошибка парсинга XML".to_string()],
    };

    let mut descriptions = Vec::new();

    if let (Some(old_gen), Some(new_gen)) = (&old_config.general, &new_config.general) {
        let mut general_changes = Vec::new();
        for (key, label) in GENERAL_LABELS {
            let old_val = get_general_value(old_gen, key);
            let new_val = get_general_value(new_gen, key);
            if old_val != new_val {
                general_changes.push(format!("{}: {} → {}", label, old_val, new_val));
            }
        }
        if !general_changes.is_empty() {
            descriptions.push(format!(
                "Настройки сервера:\n{}",
                general_changes
                    .iter()
                    .map(|c| format!("  • {}", c))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
    }

    let old_users: HashMap<&str, &UserAccount> = old_config
        .users
        .as_ref()
        .map(|u| u.users.iter().map(|u| (u.username.as_str(), u)).collect())
        .unwrap_or_default();
    let new_users: HashMap<&str, &UserAccount> = new_config
        .users
        .as_ref()
        .map(|u| u.users.iter().map(|u| (u.username.as_str(), u)).collect())
        .unwrap_or_default();

    for username in old_users.keys().chain(new_users.keys()) {
        let old_u = old_users.get(username);
        let new_u = new_users.get(username);

        match (old_u, new_u) {
            (None, Some(_)) => {
                descriptions.push(format!("Создана учётная запись {}", username));
                continue;
            }
            (Some(_), None) => {
                descriptions.push(format!("Удалена учётная запись {}", username));
                continue;
            }
            (Some(old_u), Some(new_u)) => {
                let mut changes = Vec::new();

                if old_u.user_type != new_u.user_type {
                    if let (Some(old_t), Some(new_t)) = (&old_u.user_type, &new_u.user_type) {
                        if let Some((_, _, desc)) = USER_TYPE_MAP
                            .iter()
                            .find(|(o, n, _)| o == old_t && n == new_t)
                        {
                            changes.push(desc.to_string());
                        }
                    }
                }

                if old_u.password != new_u.password && old_u.password.is_some() {
                    changes.push("изменён пароль".to_string());
                }
                if old_u.note != new_u.note {
                    changes.push(format!("заметка: {}", new_u.note.as_deref().unwrap_or("")));
                }
                if old_u.init_channel != new_u.init_channel {
                    changes.push(format!(
                        "начальный канал: {}",
                        new_u.init_channel.as_deref().unwrap_or("")
                    ));
                }
                if old_u.userdata != new_u.userdata {
                    changes.push(format!(
                        "userdata: {}",
                        new_u.userdata.as_deref().unwrap_or("0")
                    ));
                }

                if old_u.user_rights != new_u.user_rights {
                    if let (Some(old_r), Some(new_r)) = (&old_u.user_rights, &new_u.user_rights) {
                        let old_rights: std::collections::HashSet<_> =
                            parse_user_rights(old_r).into_iter().collect();
                        let new_rights: std::collections::HashSet<_> =
                            parse_user_rights(new_r).into_iter().collect();
                        let added: Vec<_> =
                            new_rights.difference(&old_rights).copied().collect();
                        let removed: Vec<_> =
                            old_rights.difference(&new_rights).copied().collect();
                        if !added.is_empty() {
                            changes.push(format!("добавлены права: {}", added.join(", ")));
                        }
                        if !removed.is_empty() {
                            changes.push(format!("убраны права: {}", removed.join(", ")));
                        }
                    }
                }

                if !changes.is_empty() {
                    descriptions.push(format!(
                        "Учётная запись {}: {}",
                        username,
                        changes.join("; ")
                    ));
                }
            }
            _ => {}
        }
    }

    let old_channels: HashMap<&str, &Channel> = old_config
        .permanent_channels
        .as_ref()
        .map(|c| c.channels.iter().map(|c| (c.channel_id.as_str(), c)).collect())
        .unwrap_or_default();
    let new_channels: HashMap<&str, &Channel> = new_config
        .permanent_channels
        .as_ref()
        .map(|c| c.channels.iter().map(|c| (c.channel_id.as_str(), c)).collect())
        .unwrap_or_default();

    for ch_id in old_channels.keys().chain(new_channels.keys()) {
        let old_ch = old_channels.get(ch_id);
        let new_ch = new_channels.get(ch_id);

        match (old_ch, new_ch) {
            (None, Some(new_ch)) => {
                descriptions.push(format!(
                    "Создан канал «{}»",
                    new_ch.name.as_deref().unwrap_or("")
                ));
                continue;
            }
            (Some(old_ch), None) => {
                descriptions.push(format!(
                    "Удалён канал «{}»",
                    old_ch.name.as_deref().unwrap_or("")
                ));
                continue;
            }
            (Some(old_ch), Some(new_ch)) => {
                let mut changes = Vec::new();
                for (key, label) in CHANNEL_LABELS {
                    let old_val = get_channel_value(old_ch, key);
                    let new_val = get_channel_value(new_ch, key);
                    if old_val != new_val {
                        changes.push(format!("{}: {} → {}", label, old_val, new_val));
                    }
                }
                if !changes.is_empty() {
                    descriptions.push(format!(
                        "Канал «{}»:\n{}",
                        new_ch.name.as_deref().unwrap_or(""),
                        changes
                            .iter()
                            .map(|c| format!("  • {}", c))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ));
                }
            }
            _ => {}
        }
    }

    descriptions
}

fn get_general_value(gen: &General, key: &str) -> String {
    match key {
        "upnp" => gen.upnp.clone().unwrap_or_default(),
        "server-name" => gen.server_name.clone().unwrap_or_default(),
        "motd" => gen.motd.clone().unwrap_or_default(),
        "auto-save" => gen.auto_save.clone().unwrap_or_default(),
        "max-users" => gen.max_users.clone().unwrap_or_default(),
        "tcpport" => gen.tcpport.clone().unwrap_or_default(),
        "udpport" => gen.udpport.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

fn get_channel_value(ch: &Channel, key: &str) -> String {
    match key {
        "name" => ch.name.clone().unwrap_or_default(),
        "password" => ch.password.clone().unwrap_or_default(),
        "topic" => ch.topic.clone().unwrap_or_default(),
        "diskquota" => ch.diskquota.clone().unwrap_or_default(),
        "max-users" => ch.max_users.clone().unwrap_or_default(),
        "channel-type" => ch.channel_type.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

fn file_hash(path: &str) -> String {
    let content = fs::read(path).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&content);
    format!("{:x}", hasher.finalize())
}

async fn send_message(bot_token: &str, chat_id: &str, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    let client = reqwest::Client::new();
    let mut body = HashMap::new();
    body.insert("chat_id", chat_id.to_string());
    body.insert("text", text.to_string());
    client.post(&url).json(&body).send().await?;
    Ok(())
}

async fn send_document(bot_token: &str, chat_id: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("https://api.telegram.org/bot{}/sendDocument", bot_token);
    let file_content = fs::read(path)?;
    let file_name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("config.xml");
    let part = reqwest::multipart::Part::bytes(file_content).file_name(file_name.to_string());
    let form = reqwest::multipart::Form::new().text("chat_id", chat_id.to_string()).part("document", part);
    let client = reqwest::Client::new();
    client.post(&url).multipart(form).send().await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let config = Config {
        bot_token: std::env::var("BOT_TOKEN").expect("BOT_TOKEN not set"),
        chat_id: std::env::var("CHAT_ID").expect("CHAT_ID not set"),
        xml_path: std::env::var("XML_PATH").unwrap_or_else(|_| "/home/tt/kirill.xml".to_string()),
        check_interval: std::env::var("CHECK_INTERVAL")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap_or(5),
    };

    let prev_path = format!("{}.prev", config.xml_path);
    if !std::path::Path::new(&prev_path).exists() {
        fs::copy(&config.xml_path, &prev_path).expect("Failed to create initial snapshot");
    }

    let mut last_hash = file_hash(&config.xml_path);
    println!("Бот запущен. Мониторинг {}", config.xml_path);

    loop {
        tokio::time::sleep(Duration::from_secs(config.check_interval)).await;

        let current_hash = file_hash(&config.xml_path);
        if current_hash == last_hash {
            continue;
        }

        println!("Файл изменён, отправка обновления");
        let descriptions = describe_changes(&prev_path, &config.xml_path);
        let now = Local::now().format("%H:%M:%S").to_string();

        let summary = if descriptions.is_empty() {
            format!("Изменения на сервере в {} (без существенных изменений)", now)
        } else {
            let mut text = format!("Изменения на сервере в {}:\n\n{}", now, descriptions.join("\n\n"));
            if text.len() > 3900 {
                text = format!("{}...(сообщение обрезано)", &text[..3900]);
            }
            text
        };

        if let Err(e) = send_message(&config.bot_token, &config.chat_id, &summary).await {
            eprintln!("Ошибка отправки сообщения: {}", e);
        }

        if let Err(e) = send_document(&config.bot_token, &config.chat_id, &config.xml_path).await {
            eprintln!("Ошибка отправки файла: {}", e);
        }

        fs::copy(&config.xml_path, &prev_path).expect("Failed to update snapshot");
        last_hash = current_hash;
    }
}
