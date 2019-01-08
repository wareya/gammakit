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
            continue;
        }
        match chars.pop_front()
        {
            Some('\\') => ret.push(c),
            Some('n') => ret.push('\n'),
            Some('r') => ret.push('\r'),
            Some('t') => ret.push('\t'),
            Some('"') => ret.push('"'),
            _ => ret.extend(&[c, c2])
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

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub (crate) enum MiniStr {
    Short([u8; 8]),
    Long(String)
}

impl MiniStr {
    pub (crate) fn from(text : &str) -> MiniStr
    {
        if text.len() <= 8
        {
            let mut ret : [u8; 8] = [0,0,0,0,0,0,0,0];
            for (i, c) in text.bytes().enumerate()
            {
                ret[i] = c;
            }
            MiniStr::Short(ret)
        }
        else
        {
            MiniStr::Long(text.to_string())
        }
    }
    #[allow(clippy::wrong_self_convention)]
    pub (crate) fn to_string(self) -> String
    {
        match self
        {
            MiniStr::Short(bytes) => std::str::from_utf8(&bytes).map(|x| x.to_string()).unwrap_or_else(|_| "<err>".to_string()),
            MiniStr::Long(string) => string
        }
    }
}
