use std::fs::File;
use std::io::Write;

use comrak::{markdown_to_html, ComrakOptions};
use serde_json;

use super::model::*;

pub trait ToJson {
    fn to_json(self) -> String;
}

pub trait ToMarkdown {
    fn to_markdown(self) -> String;
}

pub trait ToHtml {
    fn to_html(self) -> String;
}

pub struct StdOutWriter {}

impl StdOutWriter {
    pub fn new() -> StdOutWriter {
        return StdOutWriter {};
    }

    pub fn save(self, value: String) -> Result<(), i32> {
        println!("{}", value);
        return Ok(());
    }
}

pub struct FileWriter {
    pub path: String,
}

impl FileWriter {
    pub fn new(path: String) -> FileWriter {
        return FileWriter { path };
    }

    pub fn save(self, value: String) -> Result<(), i32> {
        let bytes = value.as_bytes();

        let mut file = match File::create(self.path.clone()) {
            Ok(file) => file,
            Err(err) => {
                error!(
                    "Unable to create file {} because {}!",
                    self.path.clone(),
                    err
                );
                return Err(2);
            }
        };

        match file.write_all(bytes) {
            Ok(_) => {}
            Err(err) => {
                error!("Unable to write to file ({})!", err);
                return Err(3);
            }
        };

        return Ok(());
    }
}

pub enum Writer {
    StdOut(StdOutWriter),
    File(FileWriter),
}

impl ToJson for HarFile {
    fn to_json(self) -> String {
        return serde_json::to_string_pretty(&self).unwrap();
    }
}

fn write_table(entries: Vec<NameValueEntry>) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("".to_string());
    lines.push("|Name|Value|".to_string());
    lines.push("|:----|:---|".to_string());

    for entry in entries {
        lines.push(format!("|`{}`|`{}`|", entry.name, entry.value));
    }
    lines.push("".to_string());

    return lines.join("\n");
}

impl ToMarkdown for HarFile {
    fn to_markdown(self) -> String {
        let mut lines: Vec<String> = Vec::new();

        for entry in self.log.entries {
            lines.push(format!(
                "# {} - `{}`",
                entry.request.method, entry.request.url
            ));
            lines.push("## Request".to_string());
            lines.push("\n### Headers".to_string());

            lines.push(write_table(entry.request.headers));

            lines.push("\n### Cookies".to_string());
            lines.push(write_table(entry.request.cookies));

            lines.push("\n### Query String".to_string());
            lines.push(write_table(entry.request.query_string));

            lines.push("## Response".to_string());
            lines.push(format!("**Status::** {}", entry.response.status));

            lines.push("\n### Headers".to_string());
            lines.push(write_table(entry.response.headers));

            lines.push("\n### Cookies".to_string());
            lines.push(write_table(entry.response.cookies));

            lines.push("\n### Content".to_string());
            lines.push(format!(
                "**Content Type:** {}\n",
                entry.response.content.mime_type.clone()
            ));
            if let Some(text) = entry.response.content.text {
                lines.push("**Body:**".to_string());
                match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(json) => {
                        let body = serde_json::to_string_pretty(&json).unwrap();
                        lines.push(format!("```\n{}\n```\n", body))
                    }
                    Err(_) => {
                        let body = text.replace("\\n", "\n");
                        lines.push(format!("```\n{}\n```\n", body));
                    }
                }
            }
        }

        return lines.join("\n");
    }
}

impl ToHtml for HarFile {
    fn to_html(self) -> String {
        let options = ComrakOptions {
            ext_table: true,
            ..ComrakOptions::default()
        };
        let rendered = markdown_to_html(&self.to_markdown(), &options);

        return format!(
            "<!DOCTYPE html>
    <html lang=\"en\">
    <head>
        <meta charset=\"utf-8\">
        <meta http-equiv=\"X-UA-Compatible\" content=\"IE=edge\">
        <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">
        <title>HAR Render</title>

        <!-- Bootstrap -->
        <link href=\"css/bootstrap.min.css\" rel=\"stylesheet\">

        <!-- HTML5 shim and Respond.js for IE8 support of HTML5 elements and media queries -->
        <!-- WARNING: Respond.js doesn't work if you view the page via file:// -->
        <!--[if lt IE 9]>
        <script src=\"https://oss.maxcdn.com/html5shiv/3.7.3/html5shiv.min.js\"></script>
        <script src=\"https://oss.maxcdn.com/respond/1.4.2/respond.min.js\"></script>
        <![endif]-->
    </head>
    <body>
        {}

        <!-- jQuery (necessary for Bootstrap's JavaScript plugins) -->
        <script src=\"https://ajax.googleapis.com/ajax/libs/jquery/1.12.4/jquery.min.js\"></script>
        <!-- Include all compiled plugins (below), or include individual files as needed -->
        <script src=\"js/bootstrap.min.js\"></script>
    </body>
    </html>",
            rendered
        );
    }
}
