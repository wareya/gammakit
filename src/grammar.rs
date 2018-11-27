use std::collections::HashSet;

use super::strings::*;
use super::parser::Parser;

#[derive(Clone)]
pub enum GrammarToken {
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
pub struct GrammarForm {
    pub tokens : Vec<GrammarToken>
}

impl GrammarForm {
    pub fn new(line : &str, parser : &mut Parser, regex_set : &mut HashSet<String>, symbol_set : &mut HashSet<String>, text_set : &mut HashSet<String>, intoken : bool) -> GrammarForm
    {
        let re = &mut parser.internal_regexes;
        let mut ret = GrammarForm { tokens : Vec::new() };
        let tokens : Vec<&str> = line.split(' ').collect();
        let tokenslen = tokens.len();
        let mut handle_operator_spec = false;
        for token in &tokens
        {
            if *token == ""
            {
                continue
            }
            if *token == ">>?"
            {
                ret.tokens.push(GrammarToken::RestIsOptional);
            }
            else if re.is_exact(r"%.+%$", token)
            {
                let bare = slice(token, 1, -1);
                if intoken && !regex_set.contains(&bare)
                {
                    regex_set.insert(bare.clone());
                    parser.regexes.push(bare.clone());
                }
                ret.tokens.push(GrammarToken::Regex(bare));
            }
            else if re.is_exact(r"\$.+\$\.\.\.(.)", token)
            {
                let separator = slice(token, -1, token.len() as i64);
                ret.tokens.push(GrammarToken::SeparatorNameList{text: slice(token, 1, -5), separator: separator.clone()});
                
                if re.is_exact(r"[^a-zA-Z0-9_ \t]+", &separator)
                {
                    if !symbol_set.contains(&separator)
                    {
                        symbol_set.insert(separator.clone());
                        parser.symbols.push(separator.clone());
                    }
                }
                else
                {
                    panic!("error: separator-list separator is not a symbol");
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
                if ret.tokens.len() == 1
                {
                    if tokenslen != 3
                    {
                        panic!("error: operator description line consists of not exactly three tokens");
                    }
                    handle_operator_spec = true;
                    break;
                }
                else
                {
                    panic!("error: operator description line is malformed (associativity sigil in wrong place)");
                }
            }
            else if slice(token, 0, 1) == "$" && token.len() > 1
            {
                panic!("error: stray $\n{}", line);
            }
            else
            {
                ret.tokens.push(GrammarToken::Plain(token.to_string()));
                if re.is_exact(r"[a-zA-Z_][a-zA-Z_0-9]*", token)
                {
                    if !text_set.contains(*token)
                    {
                        text_set.insert(token.to_string());
                        parser.texts.push(token.to_string());
                    }
                    text_set.insert(token.to_string());
                }
                else if re.is_exact(r"[^a-zA-Z0-9_]+", token)
                {
                    if !symbol_set.contains(*token)
                    {
                        symbol_set.insert(token.to_string());
                        parser.symbols.push(token.to_string());
                    }
                }
                else
                {
                    panic!("error: literal symbol `{}` does not follow the forms [a-zA-Z_][a-zA-Z_0-9]* || [^a-zA-Z0-9_]+\n{}", token, line);
                }
            }
        }
        if handle_operator_spec
        {
            assert!(tokens.len() == 3);
            let optext : String;
            if let GrammarToken::Plain(ref left) = ret.tokens[0]
            {
                optext = left.to_string();
            }
            else
            {
                panic!("error: operator associativity sigil's leftwards token is not a plain text token");
            }
            if let Ok(precedence) = tokens[2].parse::<i32>()
            {
                ret.tokens[0] = GrammarToken::Op{text: optext, assoc: if tokens[1] == r"\l" {1} else {0}, precedence};
            }
            else
            {
                panic!("error: operator precedence is not an integer\n{}", line);
            }
        }
        return ret;
    }
}
