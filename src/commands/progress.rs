use indicatif::{ProgressBar, ProgressStyle};

pub struct ProgressBarHelper {
    pb: ProgressBar,
}

pub enum ProgressBarType<'a> {
    SizedProgressBar(usize, &'a str),
    UnsizedProgressBar(&'a str),
}

impl ProgressBarHelper {
    pub fn new(p_type: ProgressBarType) -> Self {
        let is_debug = { *crate::DEBUG_LEVEL.lock().unwrap() >= 1 };

        if atty::isnt(atty::Stream::Stdout) || is_debug {
            ProgressBarHelper {
                pb: ProgressBar::hidden(),
            }
        } else {
            let template = match &p_type {
                ProgressBarType::SizedProgressBar(_, template) => template,
                ProgressBarType::UnsizedProgressBar(template) => template,
            };

            let pb = match p_type {
                ProgressBarType::SizedProgressBar(size, _) => ProgressBar::new(size as u64),
                ProgressBarType::UnsizedProgressBar(_) => ProgressBar::new_spinner(),
            };

            let spinner_style = ProgressStyle::default_spinner()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                .progress_chars("#>-")
                .template(&template);
            pb.set_style(spinner_style.clone());
            pb.enable_steady_tick(100);
            ProgressBarHelper { pb }
        }
    }

    #[allow(dead_code)]
    pub fn inc_with_message(&self, message: &str) {
        self.pb.inc(1);
        self.pb.set_message(message);
    }

    pub fn inc(&self) {
        self.pb.inc(1);
    }

    pub fn set_message(&self, message: &str) {
        self.pb.set_message(message);
    }

    pub fn done(&self) {
        self.pb.finish_and_clear();
    }
}
