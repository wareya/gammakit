use std::collections::HashMap;

use regex::Regex;

#[derive(Clone)]
pub struct RegexHolder {
    exact_regexes : HashMap<String, Result<Regex, regex::Error>>,
    regexes : HashMap<String, Result<Regex, regex::Error>>
}

impl RegexHolder {
    pub fn new() -> RegexHolder
    {
        RegexHolder { exact_regexes: HashMap::new(), regexes : HashMap::new() }
    }
    pub fn prepare_exact(&mut self, regex_text : &str)
    {
        if self.exact_regexes.contains_key(regex_text)
        {
            return;
        }
        let regex = Regex::new(&format!("^{}$", regex_text));
        self.exact_regexes.insert(regex_text.to_string(), regex);
    }
    #[allow(clippy::wrong_self_convention)]
    pub fn is_exact(&mut self, regex_text : &str, text : &str) -> bool
    {
        if let Some(regex) = self.exact_regexes.get(regex_text)
        {
            return match regex { Ok(regex) => regex.is_match(text), Err(_) => false };
        }
        let regex = Regex::new(&format!("^{}$", regex_text));
        self.exact_regexes.insert(regex_text.to_string(), regex);
        self.is_exact(regex_text, text)
    }
    pub fn is_exact_immut(& self, regex_text : &str, text : &str) -> bool
    {
        if let Some(regex) = self.exact_regexes.get(regex_text)
        {
            if let Ok(regex) = regex
            {
                return regex.is_match(text);
            }
            else
            {
                return false;
            }
        }
        else
        {
            panic!("internal error: attempted to use is_exact_immut for a regex that has not yet been cached");
        }
    }
    /*
    // regex offsets are bytes:
    let mystr = "あそこだよっ！";
    println!("{}", mystr);
    let re = Regex::new("[あそ]").unwrap();
    assert!(re.find_at(mystr, 0).unwrap().start() == 0);
    assert!(re.find_at(mystr, 3).unwrap().start() == 3);
    */
    pub fn match_at(&mut self, regex_text : &str, text : &str, start : usize) -> Option<String>
    {
        if let Some(regex) = self.regexes.get(regex_text)
        {
            if let Ok(regex) = regex
            {
                if let Some(my_match) = regex.find_at(text, start)
                {
                    if my_match.start() == start
                    {
                        return Some(my_match.as_str().to_string());
                    }
                }
            }
        }
        else
        {
            let regex = Regex::new(regex_text);
            self.regexes.insert(regex_text.to_string(), regex);
            return self.match_at(regex_text, text, start);
        }
        
        None
    }
}