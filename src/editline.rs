use std::{
    io::{self, Error, Write},
    sync::Mutex,
};

static HISTORY_LIST: Mutex<Vec<String>> = Mutex::new(vec![]);

pub fn readline(prefix: &str) -> Result<String, Error> {
    io::stdout().write(prefix.as_bytes())?;
    io::stdout().flush()?;

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;

    Ok(line.trim().to_string())
}

//TODO: handle mutex error
pub fn add_history(line: &str) {
    HISTORY_LIST.lock().unwrap().push(line.to_string());
}

//TODO: handle mutex error
pub fn history_list() -> Vec<String> {
    HISTORY_LIST.lock().unwrap().clone()
}

//TODO: Add tests
