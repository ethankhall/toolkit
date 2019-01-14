#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HarFile {
    pub log: LogEntry,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NameVersionEntry {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NameValueEntry {
    pub name: String,
    pub value: String,
}

#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PagesEntry {
    pub started_date_time: String,
    pub id: String,
    pub title: String,
    pub page_timings: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LogEntry {
    pub version: String,
    pub creator: NameVersionEntry,
    pub browser: Option<NameVersionEntry>,
    pub pages: Vec<PagesEntry>,
    pub entries: Vec<RequestWrapper>,
}

#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestEntry {
    pub body_size: i32,
    pub method: String,
    pub url: String,
    pub http_version: String,
    pub headers: Vec<NameValueEntry>,
    pub cookies: Vec<NameValueEntry>,
    pub query_string: Vec<NameValueEntry>,
    pub headers_size: i32,
}
#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Content {
    pub mime_type: String,
    pub size: i64,
    pub text: Option<String>,
}

#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseEntry {
    pub status: i32,
    pub status_text: String,
    pub http_version: String,
    pub headers: Vec<NameValueEntry>,
    pub cookies: Vec<NameValueEntry>,
    pub content: Content,
    #[serde(rename = "redirectURL")]
    pub redirect_url: serde_json::Value,
    pub headers_size: serde_json::Value,
    pub body_size: serde_json::Value,
}

#[serde(rename_all = "camelCase")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestWrapper {
    pub pageref: Option<String>,
    pub started_date_time: String,
    pub request: RequestEntry,
    pub response: ResponseEntry,
    pub cache: serde_json::Value,
    pub timings: serde_json::Value,
    pub time: serde_json::Value,
}
