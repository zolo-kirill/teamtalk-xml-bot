import os
import hashlib
import asyncio
import logging
import xml.etree.ElementTree as ET
from datetime import datetime
from telegram import Bot

BOT_TOKEN = "8961029801:AAEMDqSTEeQVhlzd9gNBUpATNeDug_ehXig"
CHAT_ID = "1789080411"
XML_PATH = "/home/tt/kirill.xml"
PREV_PATH = "/home/tt/kirill.xml.prev"
CHECK_INTERVAL = 5

logging.basicConfig(
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    level=logging.INFO,
)
logger = logging.getLogger(__name__)

USER_TYPE_MAP = {
    ("0", "1"): "учётная запись активирована",
    ("1", "0"): "учётная запись отключена",
    ("1", "2"): "назначена администратором",
    ("2", "1"): "сняты права администратора",
    ("0", "2"): "назначена администратором",
    ("2", "0"): "учётная запись отключена",
}

GENERAL_LABELS = {
    "upnp": "UPnP",
    "server-name": "Название сервера",
    "motd": "Сообщение дня",
    "auto-save": "Автосохранение",
    "max-users": "Макс. пользователей",
    "login-attempts": "Попытки входа",
    "max-logins-per-ipaddr": "Макс. входов с IP",
    "login-delay-msec": "Задержка входа (мс)",
    "user-timeout": "Таймаут пользователя (с)",
    "tcpport": "TCP порт",
    "udpport": "UDP 端口",
    "voicetx-limit": "Лимит передачи голоса",
    "vidcaptx-limit": "Лимит передачи видео",
    "mediafiletx-limit": "Лимит передачи медиа",
    "desktoptx-limit": "Лимит передачи рабочего стола",
    "totaltx-limit": "Общий лимит передачи",
}

USER_RIGHTS_MAP = {
    0x00000001: "множественный вход",
    0x00000002: "просмотр всех пользователей",
    0x00000004: "создание временных каналов",
    0x00000008: "изменение каналов",
    0x00000010: "широковещательные сообщения",
    0x00000020: "исключение пользователей",
    0x00000040: "блокировка пользователей",
    0x00000080: "перемещение пользователей",
    0x00000100: "включение операторского режима",
    0x00000200: "загрузка файлов",
    0x00000400: "скачивание файлов",
    0x00000800: "обновление настроек сервера",
    0x00001000: "передача голоса",
    0x00002000: "передача видео",
    0x00004000: "передача рабочего стола",
    0x00008000: "удалённое управление рабочим столом",
    0x00010000: "передача аудиофайлов",
    0x00020000: "передача видеофайлов",
    0x00040000: "фиксация никнейма",
    0x00080000: "фиксация статуса",
    0x00100000: "запись голоса",
    0x00200000: "просмотр скрытых каналов",
    0x00400000: "личные сообщения",
    0x00800000: "сообщения в канале",
}

CHANNEL_LABELS = {
    "name": "Название",
    "password": "Пароль",
    "topic": "Тема",
    "diskquota": "Дисковая квота",
    "op-password": "Пароль оператора",
    "max-users": "Макс. пользователей",
    "channel-type": "Тип канала",
    "userdata": "Данные пользователя",
    "transmit-delay-msec": "Задержка передачи (мс)",
}

BAN_LABELS = {
    "ipaddr": "IP-адрес",
    "username": "Имя пользователя",
    "bantype": "Тип блокировки",
    "bantime": "Время блокировки",
    "owner": "Кто заблокировал",
}

FILE_STORAGE_LABELS = {
    "max-diskusage": "Макс. размер диска",
    "channel-diskquota": "Квота канала",
    "files-root": "Папка файлов",
}

LOGGING_LABELS = {
    "server-log-maxsize": "Макс. размер лога",
    "server-log-events": "События лога",
}


def get_file_hash(path):
    with open(path, "rb") as f:
        return hashlib.md5(f.read()).hexdigest()


def parse_xml(path):
    try:
        tree = ET.parse(path)
        return tree.getroot()
    except ET.ParseError as e:
        logger.error("XML parse error: %s", e)
        return None


def parse_users(root):
    users = {}
    for user in root.findall(".//user"):
        username_el = user.find("username")
        if username_el is None or not username_el.text:
            continue
        username = username_el.text
        users[username] = {}
        for child in user:
            if child.tag != "username":
                users[username][child.tag] = child.text if child.text else ""
    return users


def parse_channels(root):
    channels = {}
    for ch in root.findall(".//channel"):
        ch_id_el = ch.find("channel-id")
        if ch_id_el is None or not ch_id_el.text:
            continue
        ch_id = ch_id_el.text
        channels[ch_id] = {}
        for child in ch:
            if child.tag != "channel-id":
                channels[ch_id][child.tag] = child.text if child.text else ""
    return channels


def parse_bans(root):
    bans = {}
    for ban in root.findall(".//serverbans/banned-user"):
        ip_el = ban.find("ipaddr")
        username_el = ban.find("username")
        key = (ip_el.text if ip_el is not None else "") + "|" + (username_el.text if username_el is not None else "")
        bans[key] = {}
        for child in ban:
            bans[key][child.tag] = child.text if child.text else ""
    return bans


def parse_general(root):
    general = root.find("general")
    if general is None:
        return {}
    result = {}
    for child in general:
        if child.tag == "bandwidth-limits":
            for sub in child:
                result[sub.tag] = sub.text if sub.text else ""
        else:
            result[child.tag] = child.text if child.text else ""
    return result


def parse_file_storage(root):
    fs = root.find("file-storage")
    if fs is None:
        return {}
    result = {}
    for child in fs:
        result[child.tag] = child.text if child.text else ""
    return result


def parse_logging(root):
    lg = root.find("logging")
    if lg is None:
        return {}
    result = {}
    for child in lg:
        result[child.tag] = child.text if child.text else ""
    return result


def parse_user_rights(value):
    if value is None:
        return set()
    try:
        rights_int = int(value)
    except ValueError:
        return set()
    result = set()
    for bit, name in USER_RIGHTS_MAP.items():
        if rights_int & bit:
            result.add(name)
    return result


def describe_changes(old_path, new_path):
    old_root = parse_xml(old_path)
    new_root = parse_xml(new_path)
    if old_root is None or new_root is None:
        return ["Ошибка парсинга XML"]

    descriptions = []

    old_general = parse_general(old_root)
    new_general = parse_general(new_root)
    general_changes = []
    for key in set(old_general) | set(new_general):
        old_val = old_general.get(key)
        new_val = new_general.get(key)
        if old_val == new_val:
            continue
        label = GENERAL_LABELS.get(key, key)
        general_changes.append(f"{label}: {old_val} → {new_val}")
    if general_changes:
        descriptions.append("Настройки сервера:\n" + "\n".join(f"  • {c}" for c in general_changes))

    old_fs = parse_file_storage(old_root)
    new_fs = parse_file_storage(new_root)
    fs_changes = []
    for key in set(old_fs) | set(new_fs):
        old_val = old_fs.get(key)
        new_val = new_fs.get(key)
        if old_val == new_val:
            continue
        label = FILE_STORAGE_LABELS.get(key, key)
        fs_changes.append(f"{label}: {old_val} → {new_val}")
    if fs_changes:
        descriptions.append("Файловое хранилище:\n" + "\n".join(f"  • {c}" for c in fs_changes))

    old_lg = parse_logging(old_root)
    new_lg = parse_logging(new_root)
    lg_changes = []
    for key in set(old_lg) | set(new_lg):
        old_val = old_lg.get(key)
        new_val = new_lg.get(key)
        if old_val == new_val:
            continue
        label = LOGGING_LABELS.get(key, key)
        lg_changes.append(f"{label}: {old_val} → {new_val}")
    if lg_changes:
        descriptions.append("Логирование:\n" + "\n".join(f"  • {c}" for c in lg_changes))

    old_users = parse_users(old_root)
    new_users = parse_users(new_root)
    for username in set(old_users) | set(new_users):
        old_u = old_users.get(username, {})
        new_u = new_users.get(username, {})
        if not old_u and new_u:
            descriptions.append(f"Создана учётная запись {username}")
            continue
        if old_u and not new_u:
            descriptions.append(f"Удалена учётная запись {username}")
            continue

        changes = []
        for tag in ["user-type", "password", "note", "init-channel", "userdata"]:
            old_val = old_u.get(tag)
            new_val = new_u.get(tag)
            if old_val == new_val:
                continue
            if tag == "user-type" and (old_val, new_val) in USER_TYPE_MAP:
                changes.append(USER_TYPE_MAP[(old_val, new_val)])
            elif tag == "password":
                changes.append("изменён пароль")
            elif tag == "note":
                changes.append(f"заметка: {new_val}")
            elif tag == "init-channel":
                changes.append(f"начальный канал: {new_val}")
            elif tag == "userdata":
                changes.append(f"userdata: {new_val}")

        old_rights = parse_user_rights(old_u.get("user-rights"))
        new_rights = parse_user_rights(new_u.get("user-rights"))
        added = new_rights - old_rights
        removed = old_rights - new_rights
        if added:
            changes.append("добавлены права: " + ", ".join(sorted(added)))
        if removed:
            changes.append("убраны права: " + ", ".join(sorted(removed)))

        if changes:
            descriptions.append(f"Учётная запись {username}: " + "; ".join(changes))

    old_channels = parse_channels(old_root)
    new_channels = parse_channels(new_root)
    for ch_id in set(old_channels) | set(new_channels):
        old_ch = old_channels.get(ch_id, {})
        new_ch = new_channels.get(ch_id, {})
        if not old_ch and new_ch:
            name = new_ch.get("name", f"ID {ch_id}")
            descriptions.append(f"Создан канал «{name}»")
            continue
        if old_ch and not new_ch:
            name = old_ch.get("name", f"ID {ch_id}")
            descriptions.append(f"Удалён канал «{name}»")
            continue

        changes = []
        for tag in set(old_ch) | set(new_ch):
            old_val = old_ch.get(tag)
            new_val = new_ch.get(tag)
            if old_val == new_val:
                continue
            label = CHANNEL_LABELS.get(tag, tag)
            changes.append(f"{label}: {old_val} → {new_val}")
        if changes:
            name = new_ch.get("name", f"ID {ch_id}")
            descriptions.append(f"Канал «{name}»:\n" + "\n".join(f"  • {c}" for c in changes))

    old_bans = parse_bans(old_root)
    new_bans = parse_bans(new_root)
    for key in set(old_bans) | set(new_bans):
        old_b = old_bans.get(key, {})
        new_b = new_bans.get(key, {})
        if not old_b and new_b:
            ip = new_b.get("ipaddr", "")
            username = new_b.get("username", "")
            if username:
                descriptions.append(f"Заблокирован пользователь {username}")
            elif ip:
                descriptions.append(f"Заблокирован IP {ip}")
            else:
                descriptions.append("Добавлена блокировка")
            continue
        if old_b and not new_b:
            ip = old_b.get("ipaddr", "")
            username = old_b.get("username", "")
            if username:
                descriptions.append(f"Разблокирован пользователь {username}")
            elif ip:
                descriptions.append(f"Разблокирован IP {ip}")
            else:
                descriptions.append("Удалена блокировка")

    return descriptions


async def main():
    bot = Bot(token=BOT_TOKEN)
    if not os.path.exists(PREV_PATH):
        with open(XML_PATH, "rb") as src, open(PREV_PATH, "wb") as dst:
            dst.write(src.read())
    last_hash = get_file_hash(XML_PATH)
    logger.info("Бот запущен. Мониторинг %s", XML_PATH)

    while True:
        await asyncio.sleep(CHECK_INTERVAL)
        try:
            current_hash = get_file_hash(XML_PATH)
            if current_hash != last_hash:
                logger.info("Файл изменён, отправка обновления")
                descriptions = describe_changes(PREV_PATH, XML_PATH)
                now = datetime.now().strftime("%H:%M:%S")
                if descriptions:
                    summary = f"Изменения на сервере в {now}:\n\n" + "\n\n".join(descriptions)
                else:
                    summary = f"Изменения на сервере в {now} (без существенных изменений)"
                if len(summary) > 3900:
                    summary = summary[:3900] + "\n\n... (сообщение обрезано)"
                await bot.send_message(chat_id=CHAT_ID, text=summary)
                with open(XML_PATH, "rb") as f:
                    await bot.send_document(
                        chat_id=CHAT_ID,
                        document=f,
                        filename=os.path.basename(XML_PATH),
                    )
                with open(PREV_PATH, "wb") as dst:
                    with open(XML_PATH, "rb") as src:
                        dst.write(src.read())
                last_hash = current_hash
        except Exception as e:
            logger.error("Ошибка: %s", e)
            await asyncio.sleep(10)


if __name__ == "__main__":
    asyncio.run(main())
