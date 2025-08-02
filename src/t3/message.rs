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
    Text(String),
    Image { url: String, base64: Option<String> },
}

/// Represents a message with a role, content, and unique ID.
#[derive(Clone, Debug)]
pub struct Message {
    pub id: String,
    pub role: Type,
    pub content: String,
    pub content_type: ContentType,
}

impl Message {
    /// Creates a new text `Message` with a randomly generated ID.
    ///
    /// # Arguments
    ///
    /// * `role` - Type - The role of the message sender (Assistant or User).
    /// * `content` - String - The content of the message.
    ///
    /// # Returns
    ///
    /// A new instance of `Message` with a unique ID.
    pub fn new(role: Type, content: String) -> Self {
        Self { 
            id: Uuid::new_v4().to_string(),
            role, 
            content: content.clone(),
            content_type: ContentType::Text(content),
        }
    }
    
    /// Creates a new image `Message` with a randomly generated ID.
    ///
    /// # Arguments
    ///
    /// * `role` - Type - The role of the message sender (Assistant or User).
    /// * `url` - String - The URL of the image.
    /// * `base64` - Option<String> - Optional base64 encoded image data.
    ///
    /// # Returns
    ///
    /// A new instance of image `Message` with a unique ID.
    pub fn new_image(role: Type, url: String, base64: Option<String>) -> Self {
        Self { 
            id: Uuid::new_v4().to_string(),
            role, 
            content: url.clone(),
            content_type: ContentType::Image { url, base64 },
        }
    }
    
    /// Creates a new `Message` with a specific ID.
    ///
    /// # Arguments
    ///
    /// * `id` - String - The specific ID for the message.
    /// * `role` - Type - The role of the message sender (Assistant or User).
    /// * `content` - String - The content of the message.
    ///
    /// # Returns
    ///
    /// A new instance of `Message` with the specified ID.
    pub fn with_id(id: String, role: Type, content: String) -> Self {
        Self { 
            id, 
            role, 
            content: content.clone(),
            content_type: ContentType::Text(content),
        }
    }
}