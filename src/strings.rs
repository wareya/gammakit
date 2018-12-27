use std::collections::VecDeque;

pub fn slice(text : &str, start : i64, end : i64) -> String
{
    let chars : Vec<char> = text.chars().collect();
    let u_start = if start < 0 {chars.len() - (-start as usize)} else {start as usize};
    let u_end   = if end   < 0 {chars.len() - (-end   as usize)} else {end   as usize};
    
    if u_start >= chars.len()
    {
        "".to_string()
    }
    else
    {
        chars[u_start..u_end].into_iter().collect()
    }
}

pub fn unescape(text: &str) -> String
{
    let mut ret = String::with_capacity(text.len());
    let mut chars : VecDeque<char> = text.chars().collect();
    while let Some(c) = chars.pop_front()
    {
        if c != '\\'
        {
            ret.push(c);
        }
        else if let Some(c2) = chars.pop_front()
        {
            match c2
            {
                '\\' => {ret.push(c);}
                'n' => {ret.push('\n');}
                'r' => {ret.push('\r');}
                't' => {ret.push('\t');}
                '"' => {ret.push('"');}
                _ => {ret.push(c); ret.push(c2);}
            }
        }
    }
    ret
}

pub fn escape(text: &str) -> String
{
    let mut ret = String::with_capacity(text.len());
    let mut chars : VecDeque<char> = text.chars().collect();
    while let Some(c) = chars.pop_front()
    {
        match c
        {
            '\\' => {ret.push('\\');ret.push('\\');}
            '\n' => {ret.push('\\');ret.push('n');}
            '\r' => {ret.push('\\');ret.push('r');}
            '\t' => {ret.push('\\');ret.push('t');}
            '\"' => {ret.push('\\');ret.push('"');}
            _ => {ret.push(c);}
        }
    }
    ret
}
