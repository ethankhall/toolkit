pub mod api;
pub mod post;
pub mod stats;

use std::collections::BTreeSet;

pub struct TopicUrlElement {
    pub name: String,
    pub urls: BTreeSet<String>
}

impl TopicUrlElement {
    fn new(name: String, urls: BTreeSet<String>) -> Self {
        TopicUrlElement {
            name,
            urls
        }
    }

    fn add_urls(&mut self, urls: BTreeSet<String>) {
        self.urls.extend(urls);
    }
}