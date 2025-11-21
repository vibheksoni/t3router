use uuid::Uuid;

/// Represents the role type in a message.
#[derive(Clone, Debug)]
pub enum Type {
    Assistant,
    User,
}

/// Represents the content type of a message.
#[derive(Clone, Debug)]
pub enum ContentType {
    Text,
    Image,
}

/// Represents a message with a role, content, and unique ID.
#[derive(Clone, Debug)]
pub struct Message {
    pub id: String,
    pub role: Type,
    pub content: String,
    pub content_type: ContentType,
    pub image_url: Option<String>,
    pub base64_data: Option<String>,
}

impl Message {
    ///
    /// Creates a new text `Message` with a randomly generated ID.
    ///
    /// # Arguments
    /// * `role`: `Type` - The role of the message sender.
    /// * `content`: `String` - The text content of the message.
    ///
    /// # Returns
    /// * `Message` - A new text message instance.
    pub fn new(role: Type, content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content,
            content_type: ContentType::Text,
            image_url: None,
            base64_data: None,
        }
    }

    ///
    /// Creates a new image `Message` with a randomly generated ID.
    ///
    /// # Arguments
    /// * `role`: `Type` - The role of the message sender.
    /// * `url`: `String` - The URL of the generated image.
    /// * `base64`: `Option<String>` - Optional base64-encoded image data.
    ///
    /// # Returns
    /// * `Message` - A new image message instance.
    pub fn new_image(role: Type, url: String, base64: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role,
            content: url.clone(),
            content_type: ContentType::Image,
            image_url: Some(url),
            base64_data: base64,
        }
    }

    ///
    /// Creates a new `Message` with a specific ID.
    ///
    /// # Arguments
    /// * `id`: `String` - The specific ID for the message.
    /// * `role`: `Type` - The role of the message sender.
    /// * `content`: `String` - The text content of the message.
    ///
    /// # Returns
    /// * `Message` - A new message instance with the provided ID.
    pub fn with_id(id: String, role: Type, content: String) -> Self {
        Self {
            id,
            role,
            content,
            content_type: ContentType::Text,
            image_url: None,
            base64_data: None,
        }
    }
}
