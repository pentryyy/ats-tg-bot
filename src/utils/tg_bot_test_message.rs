use teloxide::prelude::{ChatId, Message, UserId};
use teloxide::types::{
    Chat, ChatKind, ChatPrivate, FileMeta, MediaKind, MediaPhoto, MediaText, MessageCommon,
    MessageId, MessageKind, PhotoSize, User,
};

fn create_chat_data() -> Chat {
    Chat {
        id: ChatId(123),
        kind: ChatKind::Private(ChatPrivate {
            username: None,
            first_name: Some("Test".to_string()),
            last_name: None,
            bio: None,
            has_private_forwards: None,
            has_restricted_voice_and_video_messages: None,
            emoji_status_custom_emoji_id: None,
        }),
        photo: None,
        pinned_message: None,
        message_auto_delete_time: None,
        has_hidden_members: false,
        has_aggressive_anti_spam_enabled: false,
    }
}

/// Dead code методы спользуется в юнит тестах.
#[allow(dead_code)]
pub fn make_photo_test_message(file_id: &str) -> Message {
    Message {
        id: MessageId(1),
        thread_id: None,
        date: chrono::Utc::now(),
        chat: create_chat_data(),
        via_bot: None,
        kind: MessageKind::Common(MessageCommon {
            from: Some(User {
                id: UserId(123),
                is_bot: false,
                first_name: "Test".to_string(),
                last_name: None,
                username: None,
                language_code: None,
                is_premium: false,
                added_to_attachment_menu: false,
            }),
            sender_chat: None,
            author_signature: None,
            forward: None,
            reply_to_message: None,
            edit_date: None,
            media_kind: MediaKind::Photo(MediaPhoto {
                photo: vec![PhotoSize {
                    file: FileMeta {
                        id: file_id.to_string(),
                        unique_id: "".to_string(),
                        size: 100,
                    },
                    width: 100,
                    height: 100,
                }],
                caption: None,
                caption_entities: vec![],
                has_media_spoiler: false,
                media_group_id: None,
            }),
            reply_markup: None,
            is_topic_message: false,
            is_automatic_forward: false,
            has_protected_content: false,
        }),
    }
}

#[allow(dead_code)]
pub fn make_text_test_message(text: &str) -> Message {
    Message {
        id: MessageId(1),
        thread_id: None,
        date: chrono::Utc::now(),
        chat: create_chat_data(),
        via_bot: None,
        kind: MessageKind::Common(MessageCommon {
            from: Some(User {
                id: UserId(123),
                is_bot: false,
                first_name: "Test".to_string(),
                last_name: None,
                username: None,
                language_code: None,
                is_premium: false,
                added_to_attachment_menu: false,
            }),
            sender_chat: None,
            author_signature: None,
            forward: None,
            reply_to_message: None,
            edit_date: None,
            media_kind: MediaKind::Text(MediaText {
                text: text.to_string(),
                entities: vec![],
            }),
            reply_markup: None,
            is_topic_message: false,
            is_automatic_forward: false,
            has_protected_content: false,
        }),
    }
}
