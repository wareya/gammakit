use super::strings::*;
use super::parser::Parser;

#[derive(Clone)]
pub (crate) enum GrammarToken {
    Name(String),
    NameList(String),
    OptionalName(String),
    OptionalNameList(String),
    SeparatorNameList{text: String, separator: String},
    Plain(String),
    Regex(String),
    Op{text: String, assoc: i32, precedence: i32},
    RestIsOptional
}

#[derive(Clone)]
pub (crate) struct GrammarForm {
    pub (crate) tokens : Vec<GrammarToken>
}

fn plainerr<T>(mystr : &str) -> Result<T, Option<String>>
{
    Err(Some(mystr.to_string()))
}

impl GrammarForm
{
    #[allow(clippy::new_ret_no_self)]
    pub (crate) fn new(line : &str, parser : &mut Parser, intoken : bool) -> Result<GrammarForm, Option<String>>
    {
        let re = &mut parser.internal_regexes;
        let mut ret = GrammarForm { tokens : Vec::new() };
        let tokens : Vec<&str> = line.split(' ').collect();
        for token in &tokens
        {
            if *token == ""
            {
                continue;
            }
            if *token == ">>?"
            {
                ret.tokens.push(GrammarToken::RestIsOptional);
            }
            else if re.is_exact(r"%.+%$", token)
            {
                let bare = slice(token, 1, -1);
                if intoken && !parser.regex_set.contains(&bare)
                {
                    parser.regex_set.insert(bare.clone());
                    parser.regex_list.push(bare.clone());
                }
                ret.tokens.push(GrammarToken::Regex(bare));
            }
            else if re.is_exact(r"\$.+\$\.\.\.(.)", token)
            {
                let separator = slice(token, -1, token.len() as i64);
                ret.tokens.push(GrammarToken::SeparatorNameList{text: slice(token, 1, -5), separator: separator.clone()});
                
                if re.is_exact(r"[^a-zA-Z0-9_ \t]+", &separator)
                {
                    if !parser.symbol_set.contains(&separator)
                    {
                        parser.symbol_set.insert(separator.clone());
                        parser.symbol_list.push(separator.clone());
                    }
                }
                else
                {
                    return plainerr("error: separator-list separator is not a symbol");
                }
            }
            else if re.is_exact(r"\$.+\$\+", token)
            {
                ret.tokens.push(GrammarToken::NameList(slice(token, 1, -2)));
            }
            else if re.is_exact(r"\$.+\$\*", token)
            {
                ret.tokens.push(GrammarToken::OptionalNameList(slice(token, 1, -2)));
            }
            else if re.is_exact(r"\$.+\$\?", token)
            {
                ret.tokens.push(GrammarToken::OptionalName(slice(token, 1, -2)));
            }
            else if re.is_exact(r"\$.+\$", token)
            {
                ret.tokens.push(GrammarToken::Name(slice(token, 1, -1)));
            }
            else if *token == r"\l" || *token == r"\r"
            {
                if ret.tokens.len() != 1
                {
                    return plainerr("error: operator description line is malformed (associativity sigil in wrong place)");
                }
                if let Some(GrammarToken::Plain(ref left)) = ret.tokens.get(0)
                {
                    if tokens.len() != 3
                    {
                        return plainerr("error: operator description line consists of not exactly three tokens");
                    }
                    if let (Some(token), Some(sigil)) = (tokens.get(2), tokens.get(1))
                    {
                        if let Ok(precedence) = token.parse::<i32>()
                        {
                            ret.tokens = vec!(GrammarToken::Op{text: left.to_string(), assoc: if *sigil == r"\l" {1} else {0}, precedence});
                            return Ok(ret);
                        }
                        else
                        {
                            return plainerr(&format!("error: operator precedence is not an integer\n{}", line));
                        }
                    }
                    else
                    {
                        return plainerr("internal error: failed to get tokens 2 and 1 (third and second) of token list that supposedly had three tokens");
                    }
                }
                else
                {
                    return plainerr("error: operator associativity sigil's leftwards token is not a plain text token; or there was an internal error and there is no associativity sigil");
                }
            }
            else if slice(token, 0, 1) == "$" && token.len() > 1
            {
                return plainerr(&format!("error: stray $\n{}", line));
            }
            else
            {
                ret.tokens.push(GrammarToken::Plain(token.to_string()));
                if re.is_exact(r"[a-zA-Z_][a-zA-Z_0-9]*", token)
                {
                    if !parser.text_set.contains(*token)
                    {
                        parser.text_set.insert(token.to_string());
                        parser.text_list.push(token.to_string());
                    }
                }
                else if re.is_exact(r"[^a-zA-Z0-9_]+", token)
                {
                    if !parser.symbol_set.contains(*token)
                    {
                        parser.symbol_set.insert(token.to_string());
                        parser.symbol_list.push(token.to_string());
                    }
                }
                else
                {
                    return plainerr(&format!("error: literal symbol `{}` does not follow the forms [a-zA-Z_][a-zA-Z_0-9]* || [^a-zA-Z0-9_]+\n{}", token, line));
                }
            }
        }
        Ok(ret)
    }
}
