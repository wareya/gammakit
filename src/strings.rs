use std::collections::VecDeque;

pub (crate) fn slice(text : &str, start : i64, end : i64) -> String
{
    let chars : Vec<char> = text.chars().collect();
    let u_start = if start <  0 {chars.len() - (-start as usize)} else {start as usize};
    let u_end   = if end   <= 0 {chars.len() - (-end   as usize)} else {end   as usize};
    
    chars.get(u_start..u_end).map(|chars| chars.into_iter().collect()).unwrap_or_else(|| "".to_string())
}

pub (crate) fn unescape(text: &str) -> String
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
                '\\' => ret.push(c),
                'n' => ret.push('\n'),
                'r' => ret.push('\r'),
                't' => ret.push('\t'),
                '"' => ret.push('"'),
                _ => ret.extend(&[c, c2])
            }
        }
    }
    ret
}

pub (crate) fn escape(text: &str) -> String
{
    let mut ret = String::with_capacity(text.len());
    let mut chars : VecDeque<char> = text.chars().collect();
    while let Some(c) = chars.pop_front()
    {
        match c
        {
            '\\' => ret.extend(&['\\', '\\']),
            '\n' => ret.extend(&['\\', 'n']),
            '\r' => ret.extend(&['\\', 'r']),
            '\t' => ret.extend(&['\\', 't']),
            '\"' => ret.extend(&['\\', '"']),
            _ => ret.push(c)
        }
    }
    ret
}
