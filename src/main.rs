extern crate regex;
#[macro_use] extern crate maplit;

use std::fs::File;
use std::io::Read;
use std::vec::Vec;
use std::rc::Rc;
use std::collections::VecDeque;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hint::unreachable_unchecked;
use regex::Regex;
use std::time::Instant;

fn slice(text : &str, start : i64, end : i64) -> String
{
    let chars : Vec<char> = text.chars().collect();
    let u_start = if start < 0 {chars.len() - (-start as usize)} else {start as usize};
    let u_end   = if end   < 0 {chars.len() - (-end   as usize)} else {end   as usize};
    
    
    if u_start >= chars.len()
    {
        return "".to_string();
    }
    else
    {
        return chars[u_start..u_end].into_iter().collect();
    }
}

fn unescape(text: &str) -> String
{
    let mut ret = String::with_capacity(text.len());
    let mut chars : VecDeque<char> = text.chars().collect();
    while let Some(c) = chars.pop_front()
    {
        if c != '\\'
        {
            ret.push(c);
        }
        else
        {
            if let Some(c2) = chars.pop_front()
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
    }
    return ret;
}

fn escape(text: &str) -> String
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
    return ret;
}

#[derive(Clone)]
enum GrammarToken {
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
struct RegexHolder {
    exact_regexes : HashMap<String, Result<Regex, regex::Error>>,
    regexes : HashMap<String, Result<Regex, regex::Error>>
}

impl RegexHolder {
    fn new() -> RegexHolder
    {
        RegexHolder { exact_regexes: HashMap::new(), regexes : HashMap::new() }
    }
    fn prepare_exact(&mut self, regex_text : &str)
    {
        if let Some(_) = self.exact_regexes.get(regex_text)
        {
            return;
        }
        let regex = Regex::new(&format!("^{}$", regex_text));
        self.exact_regexes.insert(regex_text.to_string(), regex);
    }
    fn is_exact(&mut self, regex_text : &str, text : &str) -> bool
    {
        if let Some(regex) = self.exact_regexes.get(regex_text)
        {
            return match regex { Ok(regex) => regex.is_match(text), Err(_) => false };
        }
        let regex = Regex::new(&format!("^{}$", regex_text));
        self.exact_regexes.insert(regex_text.to_string(), regex);
        return self.is_exact(regex_text, text);
    }
    fn is_exact_immut(& self, regex_text : &str, text : &str) -> bool
    {
        if let Some(regex) = self.exact_regexes.get(regex_text)
        {
            return match regex { Ok(regex) => regex.is_match(text), Err(_) => false } ;
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
    fn match_at(&mut self, regex_text : &str, text : &str, start : usize) -> Option<String>
    {
        if let Some(regex) = self.regexes.get(regex_text)
        {
            return if let Ok(regex) = regex
            {
                if let Some(my_match) = regex.find_at(text, start)
                {
                    if my_match.start() == start
                    {
                        Some(my_match.as_str().to_string())
                    }
                    else { None }
                }
                else { None }
            }
            else { None }
        }
        let regex = Regex::new(regex_text);
        self.regexes.insert(regex_text.to_string(), regex);
        return self.match_at(regex_text, text, start);
    }
}

#[derive(Clone)]
struct GrammarForm {
    tokens : Vec<GrammarToken>
}

impl GrammarForm {
    fn new(line : String, parser : &mut Parser, regex_set : &mut HashSet<String>, symbol_set : &mut HashSet<String>, text_set : &mut HashSet<String>, intoken : bool) -> GrammarForm
    {
        let re = &mut parser.internal_regexes;
        let mut ret = GrammarForm { tokens : Vec::new() };
        let tokens : Vec<&str> = line.split(" ").collect();
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
                if intoken
                {
                    if !regex_set.contains(&bare)
                    {
                        regex_set.insert(bare.clone());
                        parser.regexes.push(bare.clone());
                    }
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
                    let thing = token.clone();
                    if !text_set.contains(thing)
                    {
                        text_set.insert(thing.to_string());
                        parser.texts.push(thing.to_string());
                    }
                    text_set.insert(token.to_string());
                }
                else if re.is_exact(r"[^a-zA-Z0-9_]+", token)
                {
                    let thing = token.clone();
                    if !symbol_set.contains(thing)
                    {
                        symbol_set.insert(thing.to_string());
                        parser.symbols.push(thing.to_string());
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
                ret.tokens[0] = GrammarToken::Op{text: optext, assoc: if tokens[1] == r"\l" {1} else {0}, precedence: precedence};
            }
            else
            {
                panic!("error: operator precedence is not an integer\n{}", line);
            }
        }
        return ret;
    }
}

#[derive(Clone)]
struct GrammarPoint {
    name: String,
    forms: Vec<GrammarForm>,
    _istoken: bool,
}

#[derive(Clone)]
struct LexToken {
    text: String,
    line: usize,
    position: usize,
}

#[derive(Clone)]
struct OpData {
    isop: bool,
    assoc: i32,
    precedence: i32
}

fn dummy_opdata() -> OpData
{
    return OpData{isop: false, assoc: 0, precedence: 0};
}

#[derive(Clone)]
struct ASTNode {
    text: String,
    line: usize,
    position: usize,
    isparent: bool,
    children: Vec<ASTNode>,
    opdata: OpData,
}

fn dummy_astnode() -> ASTNode
{
    return ASTNode{text: "".to_string(), line: 0, position: 0, isparent: false, children: Vec::new(), opdata: dummy_opdata()};
}

#[derive(Clone)]
struct Parser {
    regexes: Vec<String>,
    symbols: Vec<String>,
    texts: Vec<String>,
    nodetypemap: HashMap<String, GrammarPoint>,
    internal_regexes: RegexHolder,
    inited: bool,
}

impl Parser {
    fn new() -> Parser
    {
        Parser { regexes: Vec::new(), symbols: Vec::new(), texts: Vec::new(), nodetypemap: HashMap::new(), internal_regexes: RegexHolder::new(), inited: false }
    }
    
    fn init(&mut self, text: &String)
    {
        let start_time = Instant::now();
        
        let mut lines : VecDeque<String> = text.lines().map(|x| x.to_string()).collect();
        // guarantee the last line is ""
        lines.push_back("".to_string());
        
        // token matchers are inserted into both sets and vectors, sets to quickly check for duplicate insertion
        let mut regex_set : HashSet<String> = HashSet::new();
        let mut symbol_set : HashSet<String> = HashSet::new();
        let mut text_set : HashSet<String> = HashSet::new();
    
        while lines.len() > 0
        {
            macro_rules! pop {
                () => { unsafe { match lines.pop_front() { Some(x) => x, None => unreachable_unchecked() } } };
            }
            
            let mut line : String = pop!();
            if line == ""
            {
                continue;
            }
            else if self.internal_regexes.is_exact("(TOKEN )?[a-zA-Z_][a-zA-Z_0-9]+:", &line)
            {
                let istoken = line.starts_with("TOKEN ");
                let name = slice(&line, if istoken {6} else {0}, -1).to_string();
                // last line is guaranteed to be "" which means we are unable to pop past the end here
                let mut nodetype : GrammarPoint = GrammarPoint{name: name, forms: Vec::new(), _istoken: istoken};
                line = pop!();
                while line != ""
                {
                    nodetype.forms.push(GrammarForm::new(line, self, &mut regex_set, &mut symbol_set, &mut text_set, istoken));
                    line = pop!();
                }
                if !self.nodetypemap.contains_key(&nodetype.name)
                {
                    self.nodetypemap.insert(nodetype.name.clone(), nodetype);
                }
                else
                {
                    panic!("error: node type `{}` declared twice", nodetype.name);
                }
            }
            else
            {
                panic!("general syntax error\noffending line:\n{}", line);
            }
        }
        
        for regex in &regex_set
        {
            self.internal_regexes.prepare_exact(&regex);
        }
        
        drop(regex_set);
        drop(symbol_set);
        drop(text_set);
        
        for tuple in &self.nodetypemap
        {
            for form in &tuple.1.forms
            {
                for token in &form.tokens
                {
                    let name : String =
                    match token
                    {
                        GrammarToken::Name(text) |
                        GrammarToken::NameList(text) |
                        GrammarToken::OptionalName(text) |
                        GrammarToken::OptionalNameList(text) => {text.clone()}
                        GrammarToken::SeparatorNameList{text: name, separator: _} => {name.clone()}
                        _ => {"".to_string()}
                    };
                    if name != ""
                    {
                        if !self.nodetypemap.contains_key(&name)
                        {
                            panic!("error: node name {} is used without actually defined", name);
                        }
                    }
                }
            }
            let name = &tuple.1.name;
            let point = self.nodetypemap.get(name).unwrap();
            //if point.name == "declname"
            if false
            {
                println!("info: grammar point {} has {} forms", point.name, point.forms.len());
                for form in &point.forms
                {
                    for token in &form.tokens
                    {
                        match token
                        {
                            GrammarToken::Name(text) =>
                            { print!("n:{} ", text); }
                            GrammarToken::NameList(text) =>
                            { print!("nl:{} ", text); }
                            GrammarToken::OptionalName(text) =>
                            { print!("on:{} ", text); }
                            GrammarToken::OptionalNameList(text) =>
                            { print!("onl:{} ", text); }
                            GrammarToken::SeparatorNameList{text, separator} =>
                            { print!("snl:{}...{} ", text, separator); }
                            GrammarToken::Plain(text) =>
                            { print!("{} ", text); }
                            GrammarToken::Regex(text) =>
                            { print!("r:{} ", text); }
                            GrammarToken::Op{text, assoc : _, precedence : _} =>
                            { print!("op:{} ", text); }
                            GrammarToken::RestIsOptional => { print!(">>? "); }
                        }
                    }
                    print!("\n");
                }
                print!("\n");
            }
        }
        if !self.nodetypemap.contains_key("program")
        {
            panic!("error: grammar does not define name \"program\"");
        }
        
        self.symbols.sort_by_key(|text| -(text.len() as i64));
        self.texts  .sort_by_key(|text| -(text.len() as i64));
        
        self.inited = true;
        
        println!("init took {:?}", Instant::now().duration_since(start_time));
    }
    
    fn tokenize(&mut self, text: String, silent: bool) -> VecDeque<LexToken>
    {
        let start_time = Instant::now();
        
        let lines : Vec<String> = text.lines().map(|x| x.trim().to_string()).collect();
        
        let mut ret : VecDeque<LexToken> = VecDeque::new();
        let mut linecount = 1;
        
        for line in lines
        {
            let mut offset : usize = 0; // in bytes
            while offset < line.len() // also in bytes
            {
                let mut continue_the_while = false;
                // check for whitespace before doing anything else
                if let Some(text) = self.internal_regexes.match_at("[ \r\n\t]+", &line, offset)
                {
                    offset += text.len();
                    continue;
                }
                for rule in &self.regexes
                {
                    if let Some(text) = self.internal_regexes.match_at(&rule, &line, offset)
                    {
                        // TODO: fix position everywhere to be codepoints instead of bytes
                        ret.push_back(LexToken{text : text.clone(), line : linecount, position : offset});
                        offset += text.len();
                        continue_the_while = true;
                        break;
                    }
                }
                if continue_the_while { continue; }
                for text in &self.symbols
                {
                    if offset + text.len() > line.len() { continue; }
                    if line[offset..offset+text.len()] == *text.as_str()
                    {
                        ret.push_back(LexToken{text : text.clone(), line : linecount, position : offset});
                        offset += text.len();
                        continue_the_while = true;
                        break;
                    }
                }
                if continue_the_while { continue; }
                for text in &self.texts
                {
                    if offset + text.len() > line.len() { continue; }
                    if line[offset..offset+text.len()] == *text.as_str()
                    {
                        // don't tokenize the beginnings of names as actual names
                        if offset + text.len() + 1 > line.len()
                        {
                            if self.internal_regexes.is_exact(r"[a-zA-Z0-9_]", &slice(&line, (offset+text.len()) as i64, (offset+text.len()+1) as i64))
                            {
                                continue;
                            }
                        }
                        ret.push_back(LexToken{text : text.clone(), line : linecount, position : offset});
                        offset += text.len();
                        continue_the_while = true;
                        break;
                    }
                }
                if continue_the_while { continue; }
                panic!("failed to tokenize program\noffending line:\n{}", line);
            }
            linecount += 1;
        }
        
        if !silent
        {
            println!("lex took {:?}", Instant::now().duration_since(start_time));
        }
        
        return ret;
    }

    // attempts to parse a token list as a particular form of a grammar point
    fn parse_form(&self, tokens : &VecDeque<LexToken>, index : usize, form : &GrammarForm) -> (Option<Vec<ASTNode>>, usize)
    {
        if tokens.len() == 0
        {
            return (None, 0);
        }
        
        let mut nodes : Vec<ASTNode> = Vec::new();
        let mut totalconsumed : usize = 0;
        
        let mut defaultreturn : (Option<Vec<ASTNode>>, usize) = (None, 0);
        
        for part in &form.tokens
        {
            if tokens.len() == 0
            {
                return defaultreturn;
            }
            match part
            {
                GrammarToken::Name(text) =>
                {
                    if !self.nodetypemap.contains_key(text)
                    {
                        panic!("internal error: failed to find node type {} used by some grammar form", text);
                    }
                    let (bit, consumed) = self.parse(&tokens, index+totalconsumed, self.nodetypemap.get(text).unwrap());
                    if bit.is_some()
                    {
                        let node = bit.unwrap();
                        nodes.push(node);
                        totalconsumed += consumed;
                    }
                    else
                    {
                        return defaultreturn;
                    }
                }
                GrammarToken::NameList(text) =>
                {
                    if !self.nodetypemap.contains_key(text)
                    {
                        panic!("internal error: failed to find node type {} used by some grammar form", text);
                    }
                    let (mut bit, mut consumed) = self.parse(&tokens, index+totalconsumed, self.nodetypemap.get(text).unwrap());
                    if !bit.is_some()
                    {
                        return defaultreturn;
                    }
                    while bit.is_some()
                    {
                        let node = bit.unwrap();
                        nodes.push(node);
                        totalconsumed += consumed;
                        
                        let tuple = self.parse(&tokens, index+totalconsumed, self.nodetypemap.get(text).unwrap());
                        bit = tuple.0;
                        consumed = tuple.1;
                    }
                }
                GrammarToken::OptionalName(text) =>
                {
                    if !self.nodetypemap.contains_key(text)
                    {
                        panic!("internal error: failed to find node type {} used by some grammar form", text);
                    }
                    let (bit, consumed) = self.parse(&tokens, index+totalconsumed, self.nodetypemap.get(text).unwrap());
                    if bit.is_some()
                    {
                        let node = bit.unwrap();
                        nodes.push(node);
                        totalconsumed += consumed;
                    }
                }
                GrammarToken::OptionalNameList(text) =>
                {
                    if !self.nodetypemap.contains_key(text)
                    {
                        panic!("internal error: failed to find node type {} used by some grammar form", text);
                    }
                    let (mut bit, mut consumed) = self.parse(&tokens, index+totalconsumed, self.nodetypemap.get(text).unwrap());
                    while bit.is_some()
                    {
                        let node = bit.unwrap();
                        nodes.push(node);
                        totalconsumed += consumed;
                        
                        let tuple = self.parse(&tokens, index+totalconsumed, self.nodetypemap.get(text).unwrap());
                        bit = tuple.0;
                        consumed = tuple.1;
                    }
                }
                GrammarToken::SeparatorNameList{text, separator} =>
                {
                    if !self.nodetypemap.contains_key(text)
                    {
                        panic!("internal error: failed to find node type {} used by some grammar form", text);
                    }
                    let (mut bit, mut consumed) = self.parse(&tokens, index+totalconsumed, self.nodetypemap.get(text).unwrap());
                    if !bit.is_some()
                    {
                        return defaultreturn;
                    }
                    while bit.is_some()
                    {
                        let node = bit.unwrap();
                        nodes.push(node);
                        totalconsumed += consumed;
                        
                        if tokens.len() <= index+totalconsumed { break; }
                        let comma = tokens[index+totalconsumed].text.clone();
                        if comma != *separator { break; }
                        totalconsumed += 1;
                        
                        let tuple = self.parse(&tokens, index+totalconsumed, self.nodetypemap.get(text).unwrap());
                        bit = tuple.0;
                        consumed = tuple.1;
                        
                        // undo separator drain if right-hand rule parse failed
                        if !bit.is_some()
                        {
                            totalconsumed -= 1;
                        }
                    }
                }
                GrammarToken::Plain(text) =>
                {
                    if tokens.len() <= index+totalconsumed { return defaultreturn; }
                    let token_text = tokens[index+totalconsumed].text.clone();
                    //println!("comparing {} to {}", token_text, *text);
                    if token_text == *text
                    {
                        nodes.push(ASTNode{text : token_text.to_string(), line : tokens[index+totalconsumed].line, position : tokens[index+totalconsumed].position, isparent: false, children : Vec::new(), opdata : dummy_opdata()});
                        totalconsumed += 1;
                    }
                    else { return defaultreturn; }
                }
                GrammarToken::Regex(text) =>
                {
                    if tokens.len() <= index+totalconsumed { return defaultreturn; }
                    let token_text = tokens[index+totalconsumed].text.clone();
                    //println!("regex comparing {} to {}", token_text, *text);
                    if self.internal_regexes.is_exact_immut(text, &token_text)
                    {
                        nodes.push(ASTNode{text : token_text.to_string(), line : tokens[index+totalconsumed].line, position : tokens[index+totalconsumed].position, isparent: false, children : Vec::new(), opdata : dummy_opdata()});
                        totalconsumed += 1;
                    }
                    else { return defaultreturn; }
                }
                GrammarToken::Op{text, assoc, precedence} =>
                {
                    if tokens.len() <= index+totalconsumed { return defaultreturn; }
                    let token_text = tokens[index+totalconsumed].text.clone();
                    if token_text == *text
                    {
                        nodes.push(ASTNode{text : token_text.to_string(), line : tokens[index+totalconsumed].line, position : tokens[index+totalconsumed].position, isparent: false, children : Vec::new(), opdata : OpData{isop : true, assoc: *assoc, precedence: *precedence}});
                        totalconsumed += 1;
                    }
                    else { return defaultreturn; }
                }
                GrammarToken::RestIsOptional =>
                {
                    defaultreturn = (Some(nodes.clone()), totalconsumed);
                }
            }
        }
        
        return (Some(nodes), totalconsumed);
    }

    // attempts to parse a token list as each form of a grammar point in order and uses the first valid one
    fn parse(&self, tokens : &VecDeque<LexToken>, index : usize, nodetype : &GrammarPoint) -> (Option<ASTNode>, usize)
    {
        if tokens.len() == 0
        {
            return (None, 0);
        }
        
        for form in &nodetype.forms
        {
            let (nodes, consumed) = self.parse_form(&tokens, index, form);
            if let Some(nodes) = nodes
            {
                return (Some(ASTNode{text : nodetype.name.clone(), line : tokens[index].line, position : tokens[index].position, isparent : true, children : nodes, opdata : dummy_opdata()}), consumed);
            }
        }
        return (None, 0);
    }
    fn rotate(ast : &mut ASTNode)
    {
        assert!(ast.isparent && ast.children.len() == 3 && ast.children[2].isparent && ast.children[2].children.len() >= 1);
        let mut node_holder = dummy_astnode();
        std::mem::swap(&mut node_holder, &mut ast.children[2]); // detach right from under left (leaving dummy on left)
        std::mem::swap(&mut ast.children[2], &mut node_holder.children[0]); // move betweener from right to left (leaving dummy on right)
        std::mem::swap(ast, &mut node_holder.children[0]); // attach left to under right (leaving dummy on root)
        std::mem::swap(ast, &mut node_holder); // attach right to root
    }
    fn parse_rotate_associativity_binexpr(&self, ast : &mut ASTNode) -> bool
    {
        fn is_rotatable_binexpr(a : &ASTNode) -> bool
        {
            return a.isparent && a.children.len() == 3 && a.text.starts_with("binexpr_");
        }
        fn compatible_associativity(a : &ASTNode, b : &ASTNode) -> bool
        {
            return a.isparent && b.isparent
                && a.children[0].opdata.isop && b.children[0].opdata.isop
                && a.children[0].opdata.assoc == 1
                && b.children[0].opdata.assoc == 1
                && a.children[0].opdata.precedence == b.children[0].opdata.precedence;
        }
        if is_rotatable_binexpr(ast) && is_rotatable_binexpr(&ast.children[2]) && compatible_associativity(&ast.children[1], &ast.children[2].children[1])
        {
            Parser::rotate(ast);
            return true;
        }
        return false;
    }
    fn parse_fix_associativity(&self, ast : &mut ASTNode)
    {
        if ast.isparent
        {
            if self.parse_rotate_associativity_binexpr(ast)
            {
                self.parse_fix_associativity(ast);
            }
            else
            {
                for mut child in &mut ast.children
                {
                    self.parse_fix_associativity(&mut child);
                }
            }
        }
    }
    fn parse_tweak_ast(&self, ast : &mut ASTNode)
    {
        if ast.isparent
        {
            if ast.text == "statement" && !ast.children.last().unwrap().isparent && ast.children.last().unwrap().text == ";"
            {
                ast.children.pop();
            }
            while (ast.text.starts_with("binexpr_") || ast.text == "simplexpr" || ast.text == "supersimplexpr") && ast.children.len() == 1
            {
                // FIXME no idea if this works lol
                let mut temp = Vec::new();
                std::mem::swap(&mut temp, &mut ast.children);
                std::mem::swap(ast, &mut temp[0]);
            }
            if ast.text == "funcargs"
            {
                if ast.children.len() >= 2
                && ast.children.first().unwrap().text == "(" && !ast.children.first().unwrap().isparent
                && ast.children.last().unwrap().text == ")" && !ast.children.last().unwrap().isparent
                {
                    ast.children.pop();
                    ast.children.remove(0);
                }
            }
            if ast.text == "funccall"
            {
                if ast.children.len() == 1
                {
                    self.parse_tweak_ast(&mut ast.children[0]);
                    if ast.children[0].text == "arrayexpr"
                    {
                        panic!("error: tried to use array indexing expression as statement");
                    }
                    if ast.children[0].text == "indirection"
                    {
                        panic!("error: tried to use indirection expression as statement");
                    }
                    if ast.children[0].text != "funcexpr"
                    {
                        panic!("error: tried to use unknown expression as statement");
                    }
                    let mut temp = Vec::new();
                    std::mem::swap(&mut temp, &mut ast.children[0].children);
                    std::mem::swap(&mut temp, &mut ast.children);
                }
                while ast.children.len() > 2
                {
                    let left = ASTNode{text: "funcexpr".to_string(), line: ast.children[0].line, position: ast.children[0].position, isparent: true, children: ast.children.drain(0..2).collect(), opdata: dummy_opdata()};
                    ast.children.insert(0, left);
                }
                //return;
            }
            if match ast.text.as_str() {"rhunexpr" | "arrayref" | "funccall" => true, _ => false }
            {
                while ast.children.len() > 2
                {
                    let left = ASTNode{text: ast.text.clone(), line: ast.children[0].line, position: ast.children[0].position, isparent: true, children: ast.children.drain(0..2).collect(), opdata: dummy_opdata()};
                    ast.children.insert(0, left);
                }
            }
            
            if ast.text == "rhunexpr"
            {
                assert!(ast.children.len() >= 2);
                if ast.children[1].children[0].text == "funcargs"
                {
                    ast.text = "funcexpr".to_string();
                    let mut temp = dummy_astnode();
                    assert!(ast.children[1].children.len() == 1);
                    std::mem::swap(&mut temp, &mut ast.children[1].children[0]);
                    std::mem::swap(&mut temp, &mut ast.children[1]);
                }
                else if ast.children[1].children[0].text == "arrayindex"
                {
                    ast.text = "arrayexpr".to_string();
                    let mut temp = dummy_astnode();
                    assert!(ast.children[1].children.len() == 1);
                    std::mem::swap(&mut temp, &mut ast.children[1].children[0]);
                    std::mem::swap(&mut temp, &mut ast.children[1]);
                }
                else if ast.children[1].children[0].text == "indirection"
                {
                    ast.text = "indirection".to_string();
                    let mut temp = dummy_astnode();
                    assert!(ast.children[1].children.len() == 1);
                    std::mem::swap(&mut temp, &mut ast.children[1].children[0].children[1]);
                    std::mem::swap(&mut temp, &mut ast.children[1]);
                }
                else
                {
                    panic!("error: rhunexpr doesn't contain funcargs | arrayindex | indirection");
                }
            }
            if match ast.text.as_str() {"ifcondition" | "whilecondition" | "withstatement" => true, _ => false }
            {
                assert!(ast.children.len() >= 4);
                ast.children.remove(3);
                ast.children.remove(1);
            }
            
            for mut child in &mut ast.children
            {
                self.parse_tweak_ast(&mut child);
            }
        }
    }
    
    fn verify_ast(&self, ast : &ASTNode)
    {
        if ast.isparent
        {
            if ast.text == "objdef"
            {
                assert!(ast.children.len() >= 3);
                for child in ast.children[3..ast.children.len()-1].iter()
                {
                    if match child.children[1].children[0].text.as_str() {"create" | "destroy" => true, _ => false }
                    {
                        if child.children[3].isparent || child.children[3].text != ")"
                        {
                            panic!("error: `{}` function of object must not have any arguments", child.children[1].children[0].text);
                        }
                    }
                }
            }
            if match ast.text.as_str() {"funccall" | "funcexpr" | "arrayref" => true, _ => false }
            {
                if ast.children.len() != 2
                {
                    println!("broken ast node");
                    println!("-----");
                    print_ast_node(ast, 0);
                    println!("-----");
                    assert!(false);
                }
            }
            for child in &ast.children
            {
                self.verify_ast(&child);
            }
        }
    }
    fn parse_program(&self, tokens : &VecDeque<LexToken>, silent: bool) -> Option<ASTNode>
    {
        let start_time = Instant::now();
        
        if !silent
        {
            println!("parsing...");
        }
        let (raw_ast, consumed) = self.parse(&tokens, 0, self.nodetypemap.get("program").unwrap());
        if !silent
        {
            println!("successfully parsed {} out of {} tokens", consumed, tokens.len());
            println!("parse took {:?}", Instant::now().duration_since(start_time));
        }
        
        if consumed != tokens.len() || !raw_ast.is_some()
        {
            println!("error: unexpected or malformed expression");
            println!("(line {})", tokens.get(consumed).unwrap().line);
            println!("(position {})", tokens.get(consumed).unwrap().position);
            
            return None;
        }
        
        let mut ast = raw_ast.unwrap();
        
        if !silent
        {
            println!("fixing associativity...");
        }
        self.parse_fix_associativity(&mut ast);
        
        if !silent
        {
            println!("tweaking AST...");
        }
        self.parse_tweak_ast(&mut ast);
        
        if !silent
        {
            println!("verifying AST...");
        }
        self.verify_ast(&ast);
        
        if !silent
        {
            println!("all good!");
        }
        
        return Some(ast);
    }
}

fn print_ast_node(ast : &ASTNode, depth : usize)
{
    if depth > 0
    {
        print!("{}", " ".repeat(depth));
    }
    if ast.isparent
    {
        if ast.text == "name"
        {
            println!("name({})", ast.children[0].text);
            return;
        }
        else if ast.text == "number"
        {
            println!("number({})", ast.children[0].text);
            return;
        }
        else if ast.text == "string"
        {
            println!("string({})", ast.children[0].text);
            return;
        }
        else
        {
            println!("{} {} {}", ast.text, ast.line, ast.position);
            for child in &ast.children
            {
                print_ast_node(&child, depth+1);
            }
        }
    }
    else
    {
        println!("{}", ast.text);
    }
}

fn print_ast(ast : &ASTNode)
{
    print_ast_node(&ast, 0);
}

/*
Q: u64
q: i64
l/i: 32
h: 16
b/c: 8
f: f32
d: f64
*/

/*
fn pack_u8(num : u8) -> Vec<u8>
{
    return vec!(num);
}
fn unpack_u8(vec : &[u8]) -> u8
{
    assert!(vec.len() == 1);
    return vec[0];
}
*/

fn pack_u16(num : u16) -> Vec<u8>
{
    return vec!(((num>>8)&0xFF) as u8, (num&0xFF) as u8);
}
fn unpack_u16(vec : &[u8]) -> u16
{
    assert!(vec.len() == 2);
    return (vec[1] as u16) | ((vec[0] as u16)<<8);
}

/*
fn pack_u32(num : u32) -> Vec<u8>
{
    return vec!((num&0xFF) as u8, ((num>>8)&0xFF) as u8, ((num>>16)&0xFF) as u8, ((num>>24)&0xFF) as u8);
}
fn unpack_u32(vec : &Vec<u8>) -> u32
{
    assert!(vec.len() == 4);
    return (vec[0] as u32) | ((vec[1] as u32)<<8) | ((vec[2] as u32)<<16) | ((vec[3] as u32)<<24);
}
*/



fn pack_u64(num : u64) -> Vec<u8>
{
    return vec!(((num>>56)&0xFF) as u8, ((num>>48)&0xFF) as u8, ((num>>40)&0xFF) as u8, ((num>>32)&0xFF) as u8,
                ((num>>24)&0xFF) as u8, ((num>>16)&0xFF) as u8, ((num>>08)&0xFF) as u8, ((num>>00)&0xFF) as u8,
    );
}
fn unpack_u64(vec : &[u8]) -> u64
{
    assert!(vec.len() == 8);
    return ( vec[7] as u64) | ((vec[6] as u64)<<8) | ((vec[5] as u64)<<16) | ((vec[4] as u64)<<24)
     | ((vec[3] as u64)<<32) | ((vec[2] as u64)<<40) | ((vec[1] as u64)<<48) | ((vec[0] as u64)<<56);
}

fn pack_f64(num : f64) -> Vec<u8>
{
    let as_u64 : u64 = unsafe { std::mem::transmute(num) };
    return pack_u64(as_u64);
}

fn unpack_f64(vec : &[u8]) -> f64
{
    assert!(vec.len() == 8);
    let num = unpack_u64(vec);
    let as_f64 : f64 = unsafe { std::mem::transmute(num) };
    return as_f64;
}

fn pun_f64_as_u64(num : f64) -> u64
{
    unsafe { std::mem::transmute(num) }
}

const NOP : u8 = 0x00;
const PUSHFLT : u8 = 0x10;
const PUSHSHORT : u8 = 0x11;
const PUSHSTR : u8 = 0x12;
const PUSHVAR : u8 = 0x13;
const PUSHNAME : u8 = 0x14;
const BINOP : u8 = 0x20;
const UNOP : u8 = 0x21;
const FUNCEXPR : u8 = 0x22;
const DECLVAR : u8 = 0x30;
const DECLFAR : u8 = 0x31;
const BINSTATE : u8 = 0x32;
const UNSTATE : u8 = 0x33;
const FUNCCALL : u8 = 0x34;
const SCOPE : u8 = 0x60;
const UNSCOPE : u8 = 0x61;
const COLLECTARRAY : u8 = 0x70;
const COLLECTDICT : u8 = 0x71;
const IF : u8 = 0x80;
const IFELSE : u8 = 0x81;
const WHILE : u8 = 0x82;
const FOR : u8 = 0x83;
const WITH : u8 = 0x84;
const BREAK : u8 = 0x90;
const CONTINUE : u8 = 0x91;
const INDIRECTION : u8 = 0xA0;
const EVALUATION : u8 = 0xA1;
const ARRAYEXPR : u8 = 0xA2;
const FUNCDEF : u8 = 0xB0;
const LAMBDA : u8 = 0xB1;
const OBJDEF : u8 = 0xB2;
const EXIT : u8 = 0xF0;
const RETURN : u8 = 0xF1;
const LINENUM : u8 = 0xF8;

fn compile_astnode(ast : &ASTNode, mut scopedepth : usize) -> Vec<u8>
{
    // only used during compilation
    #[allow(non_snake_case)]
    let BINOP_TYPES : HashMap<&str, u8> = hashmap!
    { "and"=> 0x10,
      "&&" => 0x10,
      "or" => 0x11,
      "||" => 0x11,
      "==" => 0x20,
      "!=" => 0x21,
      ">=" => 0x22,
      "<=" => 0x23,
      ">"  => 0x24,
      "<"  => 0x25,
      "+"  => 0x30,
      "-"  => 0x31,
      "*"  => 0x40,
      "/"  => 0x41,
      "%"  => 0x42,
    };
    #[allow(non_snake_case)]
    let UNOP_TYPES : HashMap<&str, u8> = hashmap!
    { "-" => 0x10,
      "+" => 0x11,
      "!" => 0x20,
    };
    
    if !ast.isparent
    {
        println!("error: tried to compile non-parent ast node");
        print_ast(ast);
        assert!(false);
        unsafe { unreachable_unchecked() };
    }
    else
    {
        let mut code = Vec::<u8>::new();
        
        //println!("compiling a {} node", ast.text);
        
        if ast.text == "program"
        {
            for child in &ast.children
            {
                code.extend(compile_astnode(&child, scopedepth));
            }
            code.push(EXIT);
        }
        else if match ast.text.as_str() {"statement" | "barestatement" => true, _ => false }
        {
            code.push(LINENUM);
            code.extend(pack_u64(ast.line as u64));
            if !ast.children[0].isparent && !ast.children.last().unwrap().isparent && ast.children[0].text == "{" && ast.children.last().unwrap().text == "}"
            {
                code.push(SCOPE);
                for child in &ast.children[1..ast.children.len()-1]
                {
                    code.extend(compile_astnode(child, scopedepth+1));
                }
                code.push(UNSCOPE);
                code.extend(pack_u16(scopedepth as u16));
            }
            else if ast.children.len() == 3 && ast.children[1].isparent && ast.children[1].text == "binstateop"
            {
                let operator = &ast.children[1].children[0].text;
                code.extend(compile_astnode(&ast.children[0], scopedepth));
                code.extend(compile_astnode(&ast.children[2], scopedepth));
                code.push(BINSTATE);
                if operator == "="
                {
                    code.push(0x00);
                }
                else if match operator.as_str() { "+=" | "-=" | "*=" | "/=" => true, _ => false }
                {
                    code.push(BINOP_TYPES[slice(operator, 0, 1).as_str()]);
                }
                else
                {
                    println!("internal error: unhandled type of binary statement");
                    print_ast(ast);
                    assert!(false);
                }
            }
            else if ast.children[0].isparent
            {
                if ast.children[0].text == "withstatement"
                {
                    let ast = &ast.children[0];
                    
                    let expr = compile_astnode(&ast.children[1], scopedepth);
                    let sentinel = &ast.children[2].children[0].children[0];
                    
                    code.extend(expr);
                    code.push(WITH);
                    
                    if !sentinel.isparent && sentinel.text == "{"
                    {
                        let block = compile_astnode(&ast.children[2].children[0], scopedepth);
                        code.extend(pack_u64(block.len() as u64));
                        code.extend(block);
                    }
                    else
                    {
                        let mut block = Vec::<u8>::new();
                        block.push(SCOPE);
                        block.extend(compile_astnode(&ast.children[2].children[0], scopedepth+1));
                        block.push(UNSCOPE);
                        block.extend(pack_u16(scopedepth as u16));
                        code.extend(pack_u64(block.len() as u64));
                        code.extend(block);
                    }   
                }
                else if match ast.children[0].text.as_str() {"declaration" | "funccall" | "funcexpr" | "funcdef" | "objdef" => true , _ => false}
                {
                    code.extend(compile_astnode(&ast.children[0], scopedepth));
                }
                else if ast.children[0].text == "condition"
                {
                    code.extend(compile_astnode(&ast.children[0].children[0], scopedepth));
                }
                else if ast.children[0].text == "instruction"
                {
                    if ast.children[0].children[0].text == "break"
                    {
                        code.push(BREAK);
                    }
                    else if ast.children[0].children[0].text == "continue"
                    {
                        code.push(CONTINUE);
                    }
                    else if ast.children[0].children[0].text == "return"
                    {
                        if ast.children[0].children.len() == 2
                        {
                            code.extend(compile_astnode(&ast.children[0].children[1], scopedepth));
                        }
                        else if ast.children[0].children.len() == 1
                        {
                            code.push(PUSHFLT);
                            code.extend(pack_f64(0.0));
                        }
                        else
                        {
                            println!("internal error: broken return instruction");
                            print_ast(ast);
                            assert!(false);
                        }
                        code.push(RETURN);
                    }
                    else
                    {
                        println!("internal error: unhandled type of instruction");
                        print_ast(ast);
                        assert!(false);
                    }
                }
                else
                {
                    println!("internal error: unhandled type of statement");
                    print_ast(ast);
                    assert!(false);
                }
            }
            else
            {
                println!("internal error: statement child is not itself a parent/named node");
                print_ast(ast);
                assert!(false);
            }
        }
        else if ast.text.starts_with("binexpr_")
        {
            assert!(ast.children.len() == 3);
            code.extend(compile_astnode(&ast.children[0], scopedepth));
            code.extend(compile_astnode(&ast.children[2], scopedepth));
            code.push(BINOP);
            code.push(BINOP_TYPES[ast.children[1].children[0].text.as_str()]);
        }
        else if ast.text == "declaration"
        {
            for child in ast.children[1..].iter()
            {
                let name = &child.children[0].children[0].text;
                code.push(PUSHNAME);
                code.extend(name.bytes());
                code.push(0x00);
                if ast.children[0].text == "var"
                {
                    code.push(DECLVAR);
                }
                else if ast.children[0].text == "far"
                {
                    code.push(DECLFAR);
                }
                else
                {
                    panic!("internal error: non-var/far prefix to declaration");
                }
                if child.children.len() == 3
                {
                    code.push(PUSHNAME);
                    code.extend(name.bytes());
                    code.push(0x00);
                    code.extend(compile_astnode(&child.children[2], scopedepth));
                    code.push(BINSTATE);
                    code.push(0x00);
                }
            }
        }
        else if ast.text == "expr"
        {
            if ast.children.len() == 1
            {
                code.extend(compile_astnode(&ast.children[0], scopedepth));
            }
            else
            {
                println!("internal error: unhandled form of expression");
                print_ast(ast);
                assert!(false);
            }
        }
        else if ast.text == "simplexpr"
        {
            if ast.children.len() == 3 && !ast.children[0].isparent && !ast.children[2].isparent && ast.children[0].text == "(" && ast.children[2].text == ")"
            { 
                code.extend(compile_astnode(&ast.children[1], scopedepth));
            }
            else
            {
                println!("internal error: unhandled form of expression");
                print_ast(ast);
                assert!(false);
            }
        }
        else if ast.text == "number"
        {
            if ast.children.len() == 1
            {
                code.push(PUSHFLT);
                if let Ok(float) = ast.children[0].text.parse::<f64>()
                {
                    code.extend(pack_f64(float));
                }
                else
                {
                    println!("internal error: text cannot be converted to a floating point number by rust");
                    print_ast(ast);
                    assert!(false);
                }
            }
            else
            {
                println!("internal error: unhandled form of expression");
                print_ast(ast);
                assert!(false);
            }
        }
        else if ast.text == "string"
        {
            if ast.children.len() == 1
            {
                code.push(PUSHSTR);
                let text = slice(&ast.children[0].text, 1, -1);
                // FIXME handle \ escapes in text
                code.extend(unescape(&text).bytes());
                code.push(0x00);
            }
            else
            {
                println!("internal error: unhandled form of expression");
                print_ast(ast);
                assert!(false);
            }
        }
        else if ast.text == "name"
        {
            if ast.children.len() == 1
            {
                code.push(PUSHVAR);
                code.extend(ast.children[0].text.bytes());
                code.push(0x00);
            }
            else
            {
                println!("internal error: unhandled form of expression");
                print_ast(ast);
                assert!(false);
            }
        }
        else if ast.text == "funccall" || ast.text == "funcexpr"
        {
            if ast.children[1].children.len() > 0
            {
                let children = &ast.children[1].children[0].children;
                if children.len() > 0xFFFF
                {
                    panic!("internal error: more than 0xFFFF (around 65000) arguments to single function");
                }
                for child in children
                {
                    //print_ast(child)
                    code.extend(compile_astnode(child, scopedepth));
                }
                code.push(PUSHSHORT);
                code.extend(pack_u16(children.len() as u16))
            }
            else
            {
                code.push(PUSHSHORT);
                code.extend(pack_u16(0))
            }
            code.extend(compile_astnode(&ast.children[0], scopedepth));
            // code.push(0x00); // FIXME this was wrong
            if ast.text == "funccall"
            {
                code.push(FUNCCALL);
            }
            else
            {
                code.push(FUNCEXPR);
            }
        }
        else if ast.text == "ifcondition"
        {
            let expr = compile_astnode(&ast.children[1], scopedepth);
            let sentinel = &ast.children[2].children[0].children[0];
            let mut block : Vec<u8>;
            if !sentinel.isparent && sentinel.text == "{"
            {
                block = compile_astnode(&ast.children[2].children[0], scopedepth);
            }
            else
            {
                block = Vec::<u8>::new();
                block.push(SCOPE);
                block.extend(compile_astnode(&ast.children[2].children[0], scopedepth+1));
                block.push(UNSCOPE);
                block.extend(pack_u16(scopedepth as u16));
            }
            if ast.children.len() == 3
            {
                code.push(IF);
                code.extend(pack_u64(expr.len() as u64));
                code.extend(pack_u64(block.len() as u64));
                code.extend(expr);
                code.extend(block);
            }
            else if ast.children.len() == 5 && ast.children[3].text == "else"
            {
                let sentinel = &ast.children[4].children[0].children[0];
                let mut block2 : Vec<u8>;
                if !sentinel.isparent && sentinel.text == "{"
                {
                    block2 = compile_astnode(&ast.children[4].children[0], scopedepth);
                }
                else
                {
                    block2 = Vec::<u8>::new();
                    block2.push(SCOPE);
                    block2.extend(compile_astnode(&ast.children[4].children[0], scopedepth+1));
                    block2.push(UNSCOPE);
                    block2.extend(pack_u16(scopedepth as u16));
                }
                code.push(IFELSE);
                code.extend(pack_u64(expr.len() as u64));
                code.extend(pack_u64(block.len() as u64));
                code.extend(pack_u64(block2.len() as u64));
                code.extend(expr);
                code.extend(block);
                code.extend(block2);
            }
            else
            {
                println!("internal error: broken if condition");
                print_ast(ast);
                assert!(false);
            }
        }
        else if ast.text == "whilecondition"
        {
            let expr = compile_astnode(&ast.children[1], scopedepth);
            // FIXME: make this a subroutine lmao
            let sentinel = &ast.children[2].children[0].children[0];
            let mut block : Vec<u8>;
            if !sentinel.isparent && sentinel.text == "{"
            {
                block = compile_astnode(&ast.children[2].children[0], scopedepth);
            }
            else
            {
                block = Vec::<u8>::new();
                block.push(SCOPE);
                block.extend(compile_astnode(&ast.children[2].children[0], scopedepth+1));
                block.push(UNSCOPE);
                block.extend(pack_u16(scopedepth as u16))
            }
            code.push(WHILE);
            code.extend(pack_u64(expr.len() as u64));
            code.extend(pack_u64(block.len() as u64));
            code.extend(expr);
            code.extend(block);
        }
        else if ast.text == "forcondition"
        {
            let mut header_nodes : Vec<Option<&ASTNode>> = vec!(None, None, None);
            let mut header_index = 0;
            for node in ast.children[2..ast.children.len()-2].iter()
            {
                if node.isparent
                {
                    header_nodes[header_index] = Some(&node);
                }
                else if !node.isparent && node.text == ";"
                {
                    header_index += 1;
                }
            }
            
            // FOR loops need an extra layer of scope around them if they have an init statement
            if let Some(ref init) = header_nodes[0]
            {
                code.push(SCOPE);
                scopedepth += 1;
                code.extend(compile_astnode(&init, scopedepth));
            }
            
            // FIXME: expr needs to just test true if it's zero length
            let expr = if let Some(ref expr) = header_nodes[1] {compile_astnode(&expr, scopedepth)} else {Vec::<u8>::new()};
            
            let mut block : Vec<u8>;
            let post : Vec<u8>;
            
            // FIXME: make this a subroutine lmao
            let sentinel = &ast.children.last().unwrap().children[0].children[0];
            if !sentinel.isparent && sentinel.text == "{"
            {
                block = compile_astnode(&ast.children.last().unwrap().children[0], scopedepth);
                post = if let Some(ref body) = header_nodes[2] {compile_astnode(&body, scopedepth)} else {Vec::<u8>::new()};
            }
            else
            {
                block = Vec::<u8>::new();
                block.push(SCOPE);
                block.extend(compile_astnode(&ast.children.last().unwrap().children[0], scopedepth+1));
                post = if let Some(ref body) = header_nodes[2] {compile_astnode(&body, scopedepth+1)} else {Vec::<u8>::new()};
                block.push(UNSCOPE);
                block.extend(pack_u16(scopedepth as u16));
            }
            code.push(FOR);
            code.extend(pack_u64(expr.len() as u64));
            code.extend(pack_u64(post.len() as u64));
            code.extend(pack_u64(block.len() as u64));
            code.extend(expr);
            code.extend(post);
            code.extend(block);
            
            // FOR loops need an extra layer of scope around them if they have an init statement
            if let Some(ref _init) = header_nodes[0]
            {
                scopedepth -= 1;
                code.push(UNSCOPE);
                code.extend(pack_u16(scopedepth as u16));
            }
        }
        else if ast.text == "lvar"
        {
            if ast.children.len() == 1
            {
                if ast.children[0].text == "name"
                {
                    code.push(PUSHNAME);
                    code.extend(ast.children[0].children[0].text.bytes());
                    code.push(0x00);
                }
                else
                {
                    code.extend(compile_astnode(&ast.children[0], scopedepth))
                }
            }
            else
            {
                println!("internal error: malformed lvar reference node");
                print_ast(ast);
                assert!(false);
            }
        }
        else if ast.text == "rvar"
        {
            if ast.children.len() == 1
            {
                if ast.children[0].text == "name"
                {
                    code.push(PUSHVAR);
                    code.extend(ast.children[0].children[0].text.bytes());
                    code.push(0x00);
                }
                else
                {
                    code.extend(compile_astnode(&ast.children[0], scopedepth));;
                    if ast.children[0].isparent && match ast.children[0].text.as_str() { "indirection" | "arrayexpr" => true, _ => false }
                    {
                        code.push(EVALUATION);
                    }
                }
            }
            else
            {
                println!("internal error: malformed rvar reference node");
                print_ast(ast);
                assert!(false);
            }
        }
        else if ast.text == "funcdef"
        {
            let name = &ast.children[1].children[0].text;
            
            let mut args = Vec::<&ASTNode>::new();
            for child in ast.children[3..].iter()
            {
                if !child.isparent && child.text == ")"
                {
                    break;
                }
                args.push(&child);
            }
            
            let mut statements = Vec::<&ASTNode>::new();
            for child in ast.children[5+args.len()..].iter()
            {
                if !child.isparent && child.text == "}"
                {
                    break;
                }
                statements.push(&child);
            }
                           
            let mut argbytes = Vec::<u8>::new();
            for arg in &args
            {
                argbytes.extend(arg.children[0].text.bytes());
                argbytes.push(0x00);
            }
            
            let mut body = Vec::<u8>::new();
            for statement in &statements
            {
                body.extend(compile_astnode(&statement, 0))
            }
            body.push(EXIT);
            
            code.push(FUNCDEF);
            code.extend(name.bytes());
            code.push(0x00);
            code.extend(pack_u16(args.len() as u16));
            code.extend(pack_u64(body.len() as u64));
            code.extend(argbytes);
            code.extend(body);
        }
        else if ast.text == "lambda"
        {
            let mut  captures = Vec::<&ASTNode>::new();
            for child in ast.children[0].children[1..ast.children[0].children.len()-1].iter()
            {
                captures.push(&child);
            }
            
            let mut args = Vec::<&ASTNode>::new();
            for child in ast.children[1].children[1..ast.children[1].children.len()-1].iter()
            {
                args.push(&child);
            }
            
            let mut  statements = Vec::<&ASTNode>::new();
            for child in ast.children[2].children[1..ast.children[2].children.len()-1].iter()
            {
                statements.push(&child);
            }
                           
            let mut argbytes = Vec::<u8>::new();
            for arg in &args
            {
                argbytes.extend(arg.children[0].text.bytes());
                argbytes.push(0x00);
            }
            
            let mut body = Vec::<u8>::new();
            for statement in &statements
            {
                body.extend(compile_astnode(statement, 0))
            }
                    
            body.push(EXIT);
            
            let mut capturebytes = Vec::<u8>::new();
            for capture in &captures
            {
                capturebytes.push(PUSHSTR);
                capturebytes.extend(capture.children[0].children[0].text.bytes());
                capturebytes.push(0x00);
                capturebytes.extend(compile_astnode(&capture.children[2], scopedepth));
            }
            
            code.extend(capturebytes);
            code.push(LAMBDA);
            code.extend(pack_u16(captures.len() as u16));
            code.extend(pack_u16(args.len() as u16));
            code.extend(pack_u64(body.len() as u64));
            code.extend(argbytes);
            code.extend(body);
        }
        else if ast.text == "objdef"
        {
            let funcs = &ast.children[3..ast.children.len()-1];
            let mut childcode = Vec::<u8>::new();
            for child in funcs.iter()
            {
                childcode.extend(compile_astnode(&child, scopedepth)[1..].iter()) // cut off the FUNCDEF byte
            }
            code.push(OBJDEF);
            code.extend(ast.children[1].children[0].text.bytes());
            code.push(0x00);
            code.extend(pack_u16(funcs.len() as u16));
            code.extend(childcode);
        }
        else if ast.text == "arraybody"
        {
            let mut elementcount = 0;
            let mut childexprs = Vec::<u8>::new();
            for expression in ast.children[1..ast.children.len()-1].iter()
            {
                if expression.text == "unusedcomma"
                {
                    break
                }
                childexprs.extend(compile_astnode(&expression, scopedepth));
                elementcount += 1;
            }
            code.extend(childexprs);
            code.push(COLLECTARRAY);
            code.extend(pack_u16(elementcount as u16));
        }
        else if ast.text == "dictbody"
        {
            let mut elementcount = 0;
            let mut childexprs = Vec::<u8>::new();
            for expression in ast.children[1..ast.children.len()-1].iter()
            {
                if expression.text == "unusedcomma"
                {
                    break;
                }
                childexprs.extend(compile_astnode(&expression.children[0], scopedepth));
                childexprs.extend(compile_astnode(&expression.children[2], scopedepth));
                elementcount += 1;
            }
            code.extend(childexprs);
            code.push(COLLECTDICT);
            code.extend(pack_u16(elementcount as u16));
        }
        else if ast.text == "arrayexpr"
        {
            if ast.children[0].isparent && ast.children[0].text == "name"
            {
                code.push(PUSHNAME);
                code.extend(ast.children[0].children[0].text.bytes());
                code.push(0x00);
            }
            else
            {
                code.extend(compile_astnode(&ast.children[0], scopedepth));
            }
            code.extend(compile_astnode(&ast.children[1].children[1], scopedepth));
            code.push(ARRAYEXPR);
        }
        else if ast.text == "indirection"
        {
            code.extend(compile_astnode(&ast.children[0], scopedepth));
            if ast.children[0].text == "indirection"
            {
                code.push(EVALUATION);
            }
            code.push(PUSHNAME);
            code.extend(ast.children[1].children[0].text.bytes());
            code.push(0x00);
            code.push(INDIRECTION);
        }
        else if ast.text == "lhunop"
        {
            if ast.children.len() == 0
            {
                //print_ast(ast)
                println!("internal error: lhunop has no children");
                print_ast(ast);
                assert!(false);
            }
            else if ast.children.len() == 1
            {
                //print_ast(ast)
                code.extend(compile_astnode(&ast.children[0], scopedepth));
            }
            else
            {
                let operator = &ast.children[0].children[0].text;
                
                //println!("op is `{}`", operator);
                
                code.extend(compile_astnode(&ast.children[1], scopedepth));
                code.push(UNOP);
                
                if let Some(op) = UNOP_TYPES.get(slice(&operator, 0, 1).as_str())
                {
                    code.push(*op);
                }
                else
                {
                    println!("internal error: unhandled type of unary expression");
                    print_ast(ast);
                    assert!(false);
                }
            }
        }
        else
        {
            println!("internal error: unhandled ast node type in compiler");
            print_ast(ast);
            assert!(false);
        }
        return code;
    }
}

fn compile_bytecode(ast : &ASTNode) -> Vec<u8>
{
    return compile_astnode(ast, 0);
}

fn disassemble_bytecode(code : &Vec<u8>, mut pc : usize, mut end : usize) -> Vec<String>
{
    let mut ret = Vec::<String>::new();
    
    if end == 0
    {
        end = code.len()
    }
    if end > code.len()
    {
        panic!("end value {} is past actual end of code {} in disassembler", end, code.len());
    }
    
    macro_rules! pull_n {
        ( $n:expr ) =>
        {
            {
                pc += $n;
                code[pc-$n..pc].iter().cloned().collect::<Vec<u8>>() as Vec<u8>
            }
        }
    }
    macro_rules! pull {
        ( ) =>
        {
            {
                pc += 1;
                code[pc-1]
            }
        }
    }
    macro_rules! put_lit {
        ( $x:expr ) =>
        {
            ret.push($x.to_string())
        }
    }
    
    macro_rules! pull_text {
        ( ) =>
        {
            {
                let mut bytes = Vec::<u8>::new();
                
                let mut c = pull!();
                while c != 0 && pc < code.len() // FIXME check if this should be < or <= (will only affect malformed bytecode, but still)
                {
                    bytes.push(c);
                    c = pull!();
                }
                
                if let Ok(res) = std::str::from_utf8(&bytes)
                {
                    escape(res)
                }
                else
                {
                    "<invalid utf-8>".to_string()
                }
            }
        }
    }
    macro_rules! pull_text_unescaped {
        ( ) =>
        {
            {
                let mut bytes = Vec::<u8>::new();
                
                let mut c = pull!();
                while c != 0 && pc < code.len() // FIXME check if this should be < or <= (will only affect malformed bytecode, but still)
                {
                    bytes.push(c);
                    c = pull!();
                }
                
                if let Ok(res) = std::str::from_utf8(&bytes)
                {
                    res.to_string()
                }
                else
                {
                    "<invalid utf-8>".to_string()
                }
            }
        }
    }
    
    while pc < end
    {
        let op = code[pc];
        pc += 1;
        
        match op
        {
            NOP =>
            {
                put_lit!("NOP");
            }
            PUSHFLT =>
            {
                let immediate = pull_n!(8);
                let num = unpack_f64(&immediate);
                ret.push(format!("PUSHFLT {}", num));
            }
            PUSHSHORT =>
            {
                let immediate = pull_n!(2);
                let num = unpack_u16(&immediate);
                ret.push(format!("PUSHSHORT {}", num));
            }
            PUSHSTR =>
            {
                ret.push(format!("PUSHSTR \"{}\"", pull_text_unescaped!()));
            }
            PUSHVAR =>
            {
                ret.push(format!("PUSHVAR \"{}\"", pull_text!()));
            }
            PUSHNAME =>
            {
                ret.push(format!("PUSHNAME \"{}\"", pull_text!()));
            }
            BINOP =>
            {
                let immediate = pull!();
                ret.push(format!("BINOP {}",
                    match immediate
                    {
                        0x10 => "&&",
                        0x11 => "||",
                        0x20 => "==",
                        0x21 => "!=",
                        0x22 => ">=",
                        0x23 => "<=",
                        0x24 => ">" ,
                        0x25 => "<" ,
                        0x30 => "+" ,
                        0x31 => "-" ,
                        0x40 => "*" ,
                        0x41 => "/" ,
                        0x42 => "%" ,
                        _ => "<unknown>"
                    }
                ));
            }
            UNOP =>
            {
                let immediate = pull!();
                ret.push(format!("UNOP {}",
                    match immediate
                    {
                        0x10 => "-",
                        0x11 => "+",
                        _ => "<unknown>"
                    }
                ));
            }
            FUNCEXPR =>
            {
                put_lit!("FUNCEXPR");
            }
            DECLVAR =>
            {
                put_lit!("DECLVAR");
            }
            DECLFAR =>
            {
                put_lit!("DECLFAR");
            }
            BINSTATE =>
            {
                let immediate = pull!();
                ret.push(format!("BINSTATE {}",
                    match immediate
                    {
                        0x00 => "=" ,
                        0x30 => "+=" ,
                        0x31 => "-=" ,
                        0x40 => "*=" ,
                        0x41 => "/=" ,
                        0x42 => "%=" ,
                        _ => "<unknown>"
                    }
                ));
            }
            UNSTATE =>
            {
                put_lit!("UNSTATE <unimplemented>");
            }
            FUNCCALL =>
            {
                put_lit!("FUNCCALL");
            }
            /*
            JUMP =>
            {
                put_lit!("JUMP <unimplemented>");
            }
            BFALSE =>
            {
                put_lit!("BFALSE <unimplemented>");
            }
            */
            SCOPE =>
            {
                put_lit!("SCOPE");
            }
            UNSCOPE =>
            {
                let immediate = pull_n!(2);
                let num = unpack_u16(&immediate);
                ret.push(format!("UNSCOPE {}", num));
            }
            COLLECTARRAY =>
            {
                let immediate = pull_n!(2);
                let num = unpack_u16(&immediate);
                ret.push(format!("COLLECTARRAY {}", num));
            }
            COLLECTDICT =>
            {
                let immediate = pull_n!(2);
                let num = unpack_u16(&immediate);
                ret.push(format!("COLLECTDICT {}", num));
            }
            IF =>
            {
                let immediate_1 = pull_n!(8);
                let immediate_2 = pull_n!(8);
                let num_1 = unpack_u64(&immediate_1) as usize;
                let num_2 = unpack_u64(&immediate_2) as usize;
                let expr_disassembly = disassemble_bytecode(code, pc, pc+num_1);
                let code_disassembly = disassemble_bytecode(code, pc+num_1, pc+num_1+num_2);
                pc += num_1;
                pc += num_2;
                put_lit!("IF");
                put_lit!("(");
                for line in expr_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!(")");
                put_lit!("{");
                for line in code_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
            }
            IFELSE =>
            {
                let immediate_1 = pull_n!(8);
                let immediate_2 = pull_n!(8);
                let immediate_3 = pull_n!(8);
                let num_1 = unpack_u64(&immediate_1) as usize;
                let num_2 = unpack_u64(&immediate_2) as usize;
                let num_3 = unpack_u64(&immediate_3) as usize;
                let expr_disassembly = disassemble_bytecode(code, pc, pc+num_1);
                let code_disassembly = disassemble_bytecode(code, pc+num_1, pc+num_1+num_2);
                let else_disassembly = disassemble_bytecode(code, pc+num_1+num_2, pc+num_1+num_2+num_3);
                pc += num_1;
                pc += num_2;
                pc += num_3;
                put_lit!("IFELSE");
                put_lit!("(");
                for line in expr_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!(")");
                put_lit!("{");
                for line in code_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
                put_lit!("{");
                for line in else_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
            }
            WHILE =>
            {
                let immediate_1 = pull_n!(8);
                let immediate_2 = pull_n!(8);
                let num_1 = unpack_u64(&immediate_1) as usize;
                let num_2 = unpack_u64(&immediate_2) as usize;
                let expr_disassembly = disassemble_bytecode(code, pc, pc+num_1);
                let code_disassembly = disassemble_bytecode(code, pc+num_1, pc+num_1+num_2);
                pc += num_1;
                pc += num_2;
                put_lit!("WHILE");
                put_lit!("(");
                for line in expr_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!(")");
                put_lit!("{");
                for line in code_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
            }
            FOR =>
            {
                let immediate_1 = pull_n!(8);
                let immediate_2 = pull_n!(8);
                let immediate_3 = pull_n!(8);
                let num_1 = unpack_u64(&immediate_1) as usize;
                let num_2 = unpack_u64(&immediate_2) as usize;
                let num_3 = unpack_u64(&immediate_3) as usize;
                let expr_disassembly = disassemble_bytecode(code, pc, pc+num_1);
                let post_disassembly = disassemble_bytecode(code, pc+num_1, pc+num_1+num_2);
                let code_disassembly = disassemble_bytecode(code, pc+num_1+num_2, pc+num_1+num_2+num_3);
                pc += num_1;
                pc += num_2;
                pc += num_3;
                put_lit!("FOR");
                put_lit!("(");
                for line in expr_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!(")");
                put_lit!("(");
                for line in post_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!(")");
                put_lit!("{");
                for line in code_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
            }
            WITH =>
            {
                let immediate = pull_n!(8);
                let num = unpack_u64(&immediate) as usize;
                let code_disassembly = disassemble_bytecode(code, pc, pc+num);
                pc += num;
                put_lit!("WITH");
                put_lit!("{");
                for line in code_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
            }
            BREAK =>
            {
                put_lit!("BREAK");
            }
            CONTINUE =>
            {
                put_lit!("CONTINUE");
            }
            INDIRECTION =>
            {
                put_lit!("INDIRECTION");
            }
            EVALUATION =>
            {
                put_lit!("EVALUATION");
            }
            ARRAYEXPR =>
            {
                put_lit!("ARRAYEXPR");
            }
            /*
            ARRAYREF =>
            {
                
            }
            */
            FUNCDEF =>
            {
                let name = pull_text!();
                let immediate_1 = pull_n!(2);
                let immediate_2 = pull_n!(8);
                let num_1 = unpack_u16(&immediate_1);
                let num_2 = unpack_u64(&immediate_2) as usize;
                ret.push(format!("FUNCDEF {}", name));
                put_lit!("(");
                for _ in 0..num_1
                {
                    ret.push(format!("    {}", pull_text!()));
                }
                put_lit!(")");
                let code_disassembly = disassemble_bytecode(code, pc, pc+num_2);
                pc += num_2;
                put_lit!("{");
                for line in code_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
            }
            LAMBDA =>
            {
                let immediate_1 = pull_n!(2);
                let immediate_2 = pull_n!(2);
                let immediate_3 = pull_n!(8);
                let num_1 = unpack_u16(&immediate_1);
                let num_2 = unpack_u16(&immediate_2);
                let num_3 = unpack_u64(&immediate_3) as usize;
                put_lit!("LAMBDA");
                ret.push(format!("[{}]", num_1));
                put_lit!("(");
                for _ in 0..num_2
                {
                    ret.push(format!("    {}", pull_text!()));
                }
                put_lit!(")");
                let code_disassembly = disassemble_bytecode(code, pc, pc+num_3);
                pc += num_3;
                put_lit!("{");
                for line in code_disassembly
                {
                    ret.push(format!("    {}", line));
                }
                put_lit!("}");
            }
            OBJDEF =>
            {
                let objname = pull_text!();
                
                let immediate = pull_n!(2);
                let num = unpack_u16(&immediate);
                
                ret.push(format!("OBJDEF {}", objname));
                put_lit!("{");
                
                for _ in 0..num
                {
                    let name = pull_text!();
                    let immediate_1 = pull_n!(2);
                    let immediate_2 = pull_n!(8);
                    let num_1 = unpack_u16(&immediate_1);
                    let num_2 = unpack_u64(&immediate_2) as usize;
                    ret.push(format!("    FUNCTION {}", name));
                    put_lit!("    (");
                    for _ in 0..num_1
                    {
                        ret.push(format!("    {}", pull_text!()));
                    }
                    put_lit!("    )");
                    let code_disassembly = disassemble_bytecode(code, pc, pc+num_2);
                    pc += num_2;
                    put_lit!("    {");
                    for line in code_disassembly
                    {
                        ret.push(format!("        {}", line));
                    }
                    put_lit!("    }");
                }
                put_lit!("}");
            }
            EXIT =>
            {
                put_lit!("EXIT");
            }
            RETURN =>
            {
                put_lit!("RETURN");
            }
            LINENUM =>
            {
                let immediate = pull_n!(8);
                let num = unpack_u64(&immediate);
                ret.push(format!("LINE {}", num));
            }
            _ =>
            {
                put_lit!("<unknown>");
            }
        }
    }
    
    return ret;
}

// inaccessible types
#[derive(Debug)]
#[derive(Clone)]
struct FuncSpec {
    varnames: Vec<String>,
    code: Rc<Vec<u8>>,
    startaddr: usize,
    //finishaddr: usize,
    fromobj: bool,
    parentobj: usize,
    forcecontext: usize,
}
struct ObjSpec {
    #[allow(unused)]
    ident: usize,
    name: String,
    functions: HashMap<String, FuncSpec>
}
struct Instance {
    objtype: usize,
    ident: usize,
    variables: HashMap<String, Value>
}

// variable types (i.e. how to access a variable as an lvalue)
#[derive(Debug)]
#[derive(Clone)]
struct ArrayVar { // for x[y]
    location: NonArrayVariable,
    indexes: VecDeque<Value>
}
#[derive(Debug)]
#[derive(Clone)]
struct IndirectVar { // for x.y
    ident: usize, // id of an instance
    name: String
}
#[derive(Debug)]
#[derive(Clone)]
struct DirectVar { // for x
    name: String
}
#[derive(Debug)]
#[derive(Clone)]
enum Variable {
    Array(ArrayVar),
    Indirect(IndirectVar),
    Direct(DirectVar)
}
#[derive(Debug)]
#[derive(Clone)]
enum NonArrayVariable {
    Indirect(IndirectVar), // x.y.z evaluates x.y before storing it as the instance identity under which to find y
    Direct(DirectVar),
    ActualArray(VecDeque<Value>) // for situations where the compiler doesn't know that EVALUATE is unnecessary, like func()[0]
}

// value types
/*
#[derive(Clone)]
struct DictS {
    list: HashMap<Value, Value>
}
*/
#[derive(Debug)]
#[derive(Clone)]
struct FuncVal {
    internal: bool,
    internalname: Option<String>,
    predefined: Option<HashMap<String, Value>>,
    userdefdata: Option<FuncSpec>
}
#[derive(Debug)]
#[derive(Clone)]
enum Value {
    Number(f64),
    Text(String),
    Array(VecDeque<Value>),
    Dict(HashMap<HashableValue, Value>),
    Func(FuncVal),
    Var(Variable),
}
#[derive(Debug)]
#[derive(Clone)]
enum HashableValue {
    Number(f64),
    Text(String),
}

fn hashval_to_val(hashval : &HashableValue) -> Value
{
    match hashval
    {
        HashableValue::Number(val) => Value::Number(*val),
        HashableValue::Text(val) => Value::Text(val.clone()),
    }
}

impl std::hash::Hash for HashableValue {
    fn hash<H: std::hash::Hasher>(&self, state : &mut H)
    {
        match self
        {
            HashableValue::Number(num) =>
            {
                pun_f64_as_u64(*num).hash(state);
            }
            HashableValue::Text(text) =>
            {
                text.hash(state);
            }
        }
    }
}
impl std::cmp::PartialEq for HashableValue {
    fn eq(&self, other : &HashableValue) -> bool
    {
        match (self, other)
        {
            (HashableValue::Number(left), HashableValue::Number(right)) =>
            {
                pun_f64_as_u64(*left) == pun_f64_as_u64(*right)
            }
            (HashableValue::Text(left), HashableValue::Text(right)) =>
            {
                left == right
            }
            _ => { false }
        }
    }
}

impl std::cmp::Eq for HashableValue { }

fn format_val(val : &Value) -> Option<String>
{
    match val
    {
        Value::Number(float) =>
        {
            Some(format!("{}", format!("{:.10}", float).trim_right_matches('0').trim_right_matches('.')))
        }
        Value::Text(string) =>
        {
            Some(format!("{}", string))
        }
        Value::Array(array) =>
        {
            let mut ret = String::new();
            ret.push_str("[");
            let mut i : usize = 0;
            for val in array
            {
                if let Value::Text(text) = val
                {
                    ret.push_str(&format!("\"{}\"", escape(text)));
                }
                else if let Some(part) = format_val(val)
                {
                    ret.push_str(&part);
                }
                else
                {
                    return None
                }
                i += 1;
                if i != array.len()
                {
                    ret.push_str(", ");
                }
            }
            ret.push_str("]");
            
            Some(ret)
        }
        Value::Dict(dict) =>
        {
            let mut ret = String::new();
            ret.push_str("{");
            let mut i : usize = 0;
            for (key, val) in dict
            {
                if let Some(part) = format_val(&hashval_to_val(key))
                {
                    ret.push_str(&part);
                    ret.push_str(": ");
                }
                else
                {
                    return None
                }
                
                if let Value::Text(text) = val
                {
                    ret.push_str(&format!("\"{}\"", escape(text)));
                }
                else if let Some(part) = format_val(val)
                {
                    ret.push_str(&part);
                }
                else
                {
                    return None
                }
                i += 1;
                if i != dict.len()
                {
                    ret.push_str(", ");
                }
            }
            ret.push_str("}");
            
            Some(ret)
        }
        _ =>
        {
            None
        }
    }
}

fn value_op_add(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(left+right))
        }
        // TODO: string and array concatenation
        _ =>
        {
            Err("types incompatible with addition".to_string())
        }
    }
}
fn value_op_subtract(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(left-right))
        }
        _ =>
        {
            Err("types incompatible with subtraction".to_string())
        }
    }
}
fn value_op_multiply(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(left*right))
        }
        _ =>
        {
            Err("types incompatible with multiplication".to_string())
        }
    }
}
fn value_op_divide(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(left/right))
        }
        _ =>
        {
            Err("types incompatible with division".to_string())
        }
    }
}
fn value_op_modulo(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(mut left), Value::Number(mut right)) =>
        {
            let negative_divisor = right < 0.0;
            if negative_divisor
            {
                right = -right;
                left = -left;
            }
            
            let outval = ((left%right)+right)%right;
            Ok(Value::Number(outval))
        }
        _ =>
        {
            Err("types incompatible with modulo".to_string())
        }
    }
}
fn float_booly(f : &f64) -> bool
{
    *f >= 0.5 // FIXME do we want to replicate this or can we get away with using f.round() != 0.0 instead?
}
fn bool_floaty(b : bool) -> f64
{
    if b {1.0} else {0.0}
}
fn value_op_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left==right)))
        }
        (Value::Text(left), Value::Text(right)) =>
        {
            Ok(Value::Number(bool_floaty(left==right)))
        }
        _ =>
        {
            Err("types incompatible with equal".to_string())
        }
    }
}
fn value_op_not_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left!=right)))
        }
        (Value::Text(left), Value::Text(right)) =>
        {
            Ok(Value::Number(bool_floaty(left!=right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with equal".to_string())
        }
    }
}
fn value_op_greater_or_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left>=right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with greater than or equal".to_string())
        }
    }
}
fn value_op_less_or_equal(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left<=right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with less than or equal".to_string())
        }
    }
}
fn value_op_greater(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left>right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with greater than".to_string())
        }
    }
}
fn value_op_less(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(left<right)))
        }
        // TODO string comparison
        _ =>
        {
            Err("types incompatible with less than".to_string())
        }
    }
}
fn value_op_and(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(float_booly(left)&&float_booly(right))))
        }
        // TODO dicts
        _ =>
        {
            Err("types incompatible with logical and".to_string())
        }
    }
}
fn value_op_or(left : &Value, right : &Value) -> Result<Value, String>
{
    match (left, right)
    {
        (Value::Number(left), Value::Number(right)) =>
        {
            Ok(Value::Number(bool_floaty(float_booly(left)||float_booly(right))))
        }
        // TODO dicts
        _ =>
        {
            Err("types incompatible with logical or".to_string())
        }
    }
}


/*
BINOP_TYPES = \
{ 0x10: "&&" , # this is not an endorsement of using && instead of and in code
  0x11: "||" , # likewise for || and or
  0x20: "==" ,
  0x21: "!=" ,
  0x22: ">=" ,
  0x23: "<=" ,
  0x24: ">"  ,
  0x25: "<"  ,
  0x30: "+"  ,
  0x31: "-"  ,
  0x40: "*"  ,
  0x41: "/"  ,
  0x42: "%"  ,
}
*/

fn get_binop_function(op : u8) -> Option<Box<Fn(&Value, &Value) -> Result<Value, String>>>
{
    macro_rules! enbox {
        ( $x:ident ) =>
        {
            Some(Box::new($x))
        }
    }
    match op
    {
        0x10 => enbox!(value_op_and),
        0x11 => enbox!(value_op_or),
        0x20 => enbox!(value_op_equal),
        0x21 => enbox!(value_op_not_equal),
        0x22 => enbox!(value_op_greater_or_equal),
        0x23 => enbox!(value_op_less_or_equal),
        0x24 => enbox!(value_op_greater),
        0x25 => enbox!(value_op_less),
        0x30 => enbox!(value_op_add),
        0x31 => enbox!(value_op_subtract),
        0x40 => enbox!(value_op_multiply),
        0x41 => enbox!(value_op_divide),
        0x42 => enbox!(value_op_modulo),
        _ => None
    }
}

fn value_op_negative(value : &Value) -> Result<Value, String>
{
    match value
    {
        Value::Number(value) =>
        {
            Ok(Value::Number(-value))
        }
        _ =>
        {
            Err("type incompatible with negation".to_string())
        }
    }
}
fn value_op_positive(value : &Value) -> Result<Value, String>
{
    match value
    {
        Value::Number(value) =>
        {
            Ok(Value::Number(*value))
        }
        _ =>
        {
            Err("type incompatible with positive".to_string())
        }
    }
}
fn value_op_not(value : &Value) -> Result<Value, String>
{
    match value
    {
        Value::Number(value) =>
        {
            Ok(Value::Number(bool_floaty(!float_booly(value))))
        }
        _ =>
        {
            Err("type incompatible with positive".to_string())
        }
    }
}

fn get_unop_function(op : u8) -> Option<Box<Fn(&Value) -> Result<Value, String>>>
{
    macro_rules! enbox {
        ( $x:ident ) =>
        {
            Some(Box::new($x))
        }
    }
    match op
    {
        0x10 => enbox!(value_op_negative),
        0x11 => enbox!(value_op_positive),
        0x20 => enbox!(value_op_not),
        // TODO: add "not" and "bitwise not"
        _ => None
    }
}

fn value_truthy(imm : &Value) -> bool
{
    match imm
    {
        Value::Number(value) =>
        {
            float_booly(value)
        }
        // TODO: string and array concatenation
        _ =>
        {
            true
        }
    }
}

fn ast_to_dict(ast : &ASTNode) -> Value
{
    let mut astdict = HashMap::<HashableValue, Value>::new();
    
    macro_rules! to_key {
        ( $str:expr ) =>
        {
            HashableValue::Text($str.to_string())
        }
    }
    
    astdict.insert(to_key!("text"), Value::Text(ast.text.clone()));
    astdict.insert(to_key!("line"), Value::Number(ast.line as f64));
    astdict.insert(to_key!("position"), Value::Number(ast.line as f64));
    astdict.insert(to_key!("isparent"), Value::Number(bool_floaty(ast.isparent)));
    
    let mut children = VecDeque::<Value>::new();
    
    for child in &ast.children
    {
        children.push_back(ast_to_dict(&child));
    }
    
    astdict.insert(to_key!("children"), Value::Array(children));
    
    let mut opdata = HashMap::<HashableValue, Value>::new();
    
    /*
    struct OpData {
        isop: bool,
        assoc: i32,
        precedence: i32
    }
    */
    
    opdata.insert(to_key!("isop"), Value::Number(bool_floaty(ast.opdata.isop)));
    opdata.insert(to_key!("assoc"), Value::Number(ast.opdata.assoc as f64));
    opdata.insert(to_key!("precedence"), Value::Number(ast.opdata.precedence as f64));
    
    astdict.insert(to_key!("opdata"), Value::Dict(opdata));
    
    return Value::Dict(astdict);
}

fn dict_to_ast(dict : &HashMap<HashableValue, Value>) -> ASTNode
{
    let mut ast = dummy_astnode();
    
    macro_rules! get {
        ( $dict:expr, $str:expr ) =>
        {
            $dict.get(&HashableValue::Text($str.to_string()))
        }
    }
    
    macro_rules! handle {
        ( $into:expr, $dict:expr, $str:expr, $strident:ident, $subtype:ident, $helper:ident, $cast:ident, $errortext:expr ) =>
        {
            if let Some(Value::$subtype($strident)) = get!($dict, $str)
            {
                $into.$strident = $strident.$helper() as $cast;
            }
            else
            {
                panic!("error: tried to turn a dict into an ast but dict lacked \"{}\" field or the \"{}\" field was not {}", $str, $str, $errortext);
            }
        }
    }
    
    handle!(ast, dict, "text", text, Text, clone, String, "a string");
    handle!(ast, dict, "line", line, Number, round, usize, "a number");
    handle!(ast, dict, "position", position, Number, round, usize, "a number");
    if let Some(Value::Number(isparent)) = get!(dict, "isparent")
    {
        ast.isparent = float_booly(isparent);
    }
    else
    {
        panic!("error: tried to turn a dict into an ast but dict lacked \"isparent\" field or the \"isparent\" field was not a number");
    }
    
    if let Some(Value::Array(val_children)) = get!(dict, "children")
    {
        // ast.children from dummy_astnode() starts out extant but empty
        for child in val_children
        {
            if let Value::Dict(dict) = child
            {
                ast.children.push(dict_to_ast(&dict));
            }
            else
            {
                panic!("error: values in list of children in ast node must be dictionaries that are themselves ast nodes");
            }
        }
    }
    else
    {
        panic!("error: tried to turn a dict into an ast but dict lacked \"children\" field or the \"children\" field was not a list");
    }
    
    if let Some(Value::Dict(val_opdata)) = get!(dict, "opdata")
    {
        if let Some(Value::Number(isop)) = get!(val_opdata, "isop")
        {
            ast.opdata.isop = float_booly(isop);
        }
        else
        {
            panic!("error: tried to turn a dict into an ast but dict's opdata lacked \"isop\" field or the \"isop\" field was not a number");
        }
        if let Some(Value::Number(assoc)) = get!(val_opdata, "assoc")
        {
            ast.opdata.assoc = assoc.round() as i32;
        }
        else
        {
            panic!("error: tried to turn a dict into an ast but dict's opdata lacked \"assoc\" field or the \"assoc\" field was not a number");
        }
        if let Some(Value::Number(precedence)) = get!(val_opdata, "precedence")
        {
            ast.opdata.precedence = precedence.round() as i32;
        }
        else
        {
            panic!("error: tried to turn a dict into an ast but dict's opdata lacked \"precedence\" field or the \"precedence\" field was not a number");
        }
    }
    else
    {
        panic!("error: tried to turn a dict into an ast but dict lacked \"opdata\" field or the \"opdata\" field was not a dictionary");
    }
    
    return ast;
}

// internal types
#[derive(Debug)]
#[derive(Clone)]
struct ControlData {
    controltype: u8,
    controlpoints: Vec<usize>,
    scopes: u16,
    other: Option<VecDeque<usize>> // in with(), a list of instance IDs
}
struct Frame {
    code: Rc<Vec<u8>>,
    pc: usize,
    scopes: Vec<HashMap<String, Value>>,
    instancestack: Vec<usize>,
    controlstack: Vec<ControlData>,
    stack: Vec<Value>,
    isexpr: bool,
    currline: usize,
}
impl Frame {
    fn new_root(code : Rc<Vec<u8>>) -> Frame
    {
        Frame { code : Rc::clone(&code), pc : 0, scopes : vec!(HashMap::<String, Value>::new()), instancestack : Vec::new(), controlstack : Vec::new(), stack : Vec::new(), isexpr : false, currline : 0 }
    }
    fn new_from_call(code : Rc<Vec<u8>>, pc : usize, isexpr : bool) -> Frame
    {
        Frame { code : Rc::clone(&code), pc, scopes : vec!(HashMap::<String, Value>::new()), instancestack : Vec::new(), controlstack : Vec::new(), stack : Vec::new(), isexpr, currline : 0 }
    }
}

// global interpreter data
struct GlobalState {
    instance_id: usize,// init 100000000
    object_id: usize,  // init 300000000
    instances: HashMap<usize, Instance>,
    instances_by_type: HashMap<usize, Vec<usize>>,
    objectnames: HashMap<String, usize>,
    objects: HashMap<usize, ObjSpec>,
    regex_holder: RegexHolder,
    parser: Parser
}

impl GlobalState {
    fn new(parser : Parser) -> GlobalState
    {
        GlobalState { instance_id : 100000000, object_id : 300000000, instances : HashMap::new(), instances_by_type : HashMap::new(), objectnames : HashMap::new(), objects : HashMap::new() , regex_holder : RegexHolder::new(), parser }
    }
}

// interpreter state
struct Interpreter {
    top_frame: Frame,
    frames: Vec<Frame>,
    doexit: bool,
    suppress_for_expr_end: bool
}

macro_rules! list_pop_generic {
    ( $list:expr, $x:ident ) =>
    {
        if let Some(val) = $list.pop()
        {
            if let Value::$x(ret) = val
            {
                Ok(ret)
            }
            else
            {
                Err(1)
            }
        }
        else
        {
            Err(0)
        }
    }
}
impl Interpreter {
    fn new(code : Vec<u8>) -> Interpreter
    {
        Interpreter { top_frame : Frame::new_root(Rc::new(code)), frames : vec!() , doexit : false, suppress_for_expr_end : false }
    }
    fn get_code(&self) -> Rc<Vec<u8>>
    {
        return Rc::clone(&self.top_frame.code);
    }
    fn get_pc(&self) -> usize
    {
        self.top_frame.pc
    }
    fn set_pc(&mut self, new : usize)
    {
        self.top_frame.pc = new;
    }
    fn add_pc(&mut self, new : usize)
    {
        self.top_frame.pc += new;
    }
    
    fn pull_from_code(&mut self, n : usize) -> Vec<u8>
    {
        let vec = self.get_code()[self.get_pc()..self.get_pc()+n].iter().cloned().collect();
        self.add_pc(n);
        return vec;
    }
    fn pull_single_from_code(&mut self) -> u8
    {
        let vec = self.get_code()[self.get_pc()];
        self.add_pc(1);
        return vec;
    }
    
    fn jump_to_function(&mut self, function : &FuncSpec, mut args : Vec<Value>, isexpr : bool, predefined : Option<HashMap<String, Value>>)
    {
        if function.varnames.len() > args.len()
        {
            panic!("error: did not provide enough arguments to function");
        }
        if function.varnames.len() < args.len()
        {
            panic!("error: provided too many arguments to function");
        }
        
        let mut newframe = Frame::new_from_call(Rc::clone(&function.code), function.startaddr, isexpr);
        std::mem::swap(&mut newframe, &mut self.top_frame);
        self.frames.push(newframe); // actually the old frame
        
        // copy lambda's universe, if there is one
        if let Some(universe) = predefined
        {
            self.top_frame.scopes = vec!(universe);
        }
        self.set_pc(function.startaddr);
        
        for varname in &function.varnames
        {
            if let Some(arg) = args.pop()
            {
                if let Some(scope) = self.top_frame.scopes.last_mut()
                {
                    scope.insert(varname.clone(), arg);
                }
                else
                {
                    panic!("internal error: no scope in top frame despite just making it in jump_to_function (this error should be unreachable!)");
                }
            }
            else
            {
                panic!("internal error: list of arguments to provide to function was shorter than list of argument names (this error should be unreachable!)");
            }
        }
    }
    fn list_pop_number(&mut self, args : &mut Vec<Value>) -> Result<f64, i32> // second val: 0: no value on stack; 1: value on stack was of the wrong type
    {
        list_pop_generic!(args, Number)
    }
    fn list_pop_text(&mut self, args : &mut Vec<Value>) -> Result<String, i32>
    {
        list_pop_generic!(args, Text)
    }
    /*
    fn list_pop_name(&mut self, args : &mut Vec<Value>) -> Result<String, i32>
    {
        let var = list_pop_generic!(args, Var)?;
        if let Variable::Direct(DirectVar{name:text}) = var
        {
            Ok(text)
        }
        else
        {
            Err(1)
        }
    }
    */
    fn list_pop_func(&mut self, args : &mut Vec<Value>) -> Result<FuncVal, i32>
    {
        list_pop_generic!(args, Func)
    }
    fn list_pop_dict(&mut self, args : &mut Vec<Value>) -> Result<HashMap<HashableValue, Value>, i32>
    {
        list_pop_generic!(args, Dict)
    }
    
    // last argument is isexpr - as of the time of writing this comment, it's used exclusively by instance_execute
    // second return value is whether the frame was moved - necessary for weird functions like instance_create that implicly call user defined functions, because moving the frame to call user defined functions also moves the original stack
    
    fn sim_func_print(&mut self, _global : &mut GlobalState, args : Vec<Value>, _ : bool) -> (Value, bool)
    {
        for arg in args
        {
            if let Some(string) = format_val(&arg)
            {
                println!("{}", string);
            }
            else
            {
                panic!("error: tried to print unprintable value");
            }
        }
        return (Value::Number(0.0), false);
    }
    fn sim_func_len(&mut self, _global : &mut GlobalState, mut args : Vec<Value>, _ : bool) -> (Value, bool)
    {
        if args.len() != 1
        {
            panic!("error: wrong number of arguments to len(); expected 1, got {}", args.len());
        }
        if let Some(arg) = args.pop()
        {
            match arg
            {
                Value::Text(string) =>
                {
                    return (Value::Number(string.chars().collect::<Vec<char>>().len() as f64), false);
                }
                Value::Array(array) =>
                {
                    return (Value::Number(array.len() as f64), false);
                }
                Value::Dict(dict) =>
                {
                    return (Value::Number(dict.keys().len() as f64), false);
                }
                _ =>
                {
                    panic!("error: tried to take length of lengthless type");
                }
            }
        }
        else
        {
            panic!("internal error: failed to read argument for len() despite having the right number of arguments (this error should be unreachable!)");
        }
    }
    fn sim_func_keys(&mut self, _global : &mut GlobalState, mut args : Vec<Value>, _ : bool) -> (Value, bool)
    {
        if args.len() != 1
        {
            panic!("error: wrong number of arguments to keys(); expected 1, got {}", args.len());
        }
        if let Some(arg) = args.pop()
        {
            match arg
            {
                Value::Array(array) =>
                {
                    let mut list = VecDeque::<Value>::new();
                    for i in 0..array.len()
                    {
                        list.push_back(Value::Number(i as f64));
                    }
                    return (Value::Array(list), false);
                }
                Value::Dict(dict) =>
                {
                    let mut list = VecDeque::<Value>::new();
                    for key in dict.keys()
                    {
                        list.push_back(hashval_to_val(key));
                    }
                    return (Value::Array(list), false);
                }
                _ =>
                {
                    panic!("error: tried to take length of lengthless type");
                }
            }
        }
        else
        {
            panic!("internal error: failed to read argument for keys() despite having the right number of arguments (this error should be unreachable!)");
        }
    }
    fn sim_func_instance_create(&mut self, global : &mut GlobalState, mut args : Vec<Value>, _ : bool) -> (Value, bool)
    {
        if args.len() != 1
        {
            panic!("error: wrong number of arguments to instance_create(); expected 1, got {}", args.len());
        }
        if let Ok(object_id_f) = self.list_pop_number(&mut args)
        {
            let object_id = object_id_f.round() as usize;
            let instance_id = global.instance_id as usize;
            if let Some(object) = global.objects.get(&object_id)
            {
                let new = Instance { objtype : object_id, ident : instance_id, variables : hashmap!{"x".to_string() => Value::Number(0.0), "y".to_string() => Value::Number(0.0)} }; // FIXME configurable default variables?
                global.instances.insert(instance_id, new); // FIXME: check for id clash
                
                let mut dumbworkaround = true;
                if let Some(ref mut instance_list) = global.instances_by_type.get_mut(&object_id)
                {
                    instance_list.push(instance_id); // gives no clash if there is no clash abovs
                    dumbworkaround = false;
                }
                if dumbworkaround
                {
                    global.instances_by_type.insert(object_id, vec!(instance_id));
                }
                
                let mut frame_moved = false;
                
                if let Some(function) = object.functions.get("create")
                {
                    self.jump_to_function(function, Vec::new(), false, None);
                    self.top_frame.instancestack.push(instance_id);
                    frame_moved = true;
                }
                
                global.instance_id += 1;
                return (Value::Number(instance_id as f64), frame_moved);
            }
            else
            {
                panic!("error: tried to create instance of non-extant object type {}", object_id);
            }
        }
        else
        {
            panic!("error: tried to use a non-number as an object id");
        }
    }
    fn sim_func_instance_add_variable(&mut self, global : &mut GlobalState, mut args : Vec<Value>, _ : bool) -> (Value, bool)
    {
        if args.len() < 2
        {
            panic!("error: wrong number of arguments to instance_add_variable(); expected 2 or more, got {}", args.len());
        }
        if let Ok(instance_id_f) = self.list_pop_number(&mut args)
        {
            let instance_id = instance_id_f.round() as usize;
            if let Ok(name) = self.list_pop_text(&mut args)
            {
                if !global.regex_holder.is_exact(r"[a-zA-Z_][a-zA-Z_0-9]*", &name)
                {
                    panic!("error: tried to create a variable with an invalid identifier `{}`\n(note: must exactly match the regex [a-zA-Z_][a-zA-Z_0-9]*)", name, )
                }
                let mut value : Value;
                if args.len() == 1
                {
                    if let Some(set_value) = args.pop()
                    {
                        value = set_value;
                    }
                    else
                    {
                        panic!("internal error: argument list was three values long but could not pop from it three times (this should be unreachable!)");
                    }
                }
                else
                {
                    value = Value::Number(0.0);
                }
                if let Some(inst) = global.instances.get_mut(&instance_id)
                {
                    if inst.variables.contains_key(&name)
                    {
                        panic!("error: tried to add variable to instance that already had a variable with that name")
                    }
                    inst.variables.insert(name, value);
                }
                else
                {
                    panic!("error: tried to add variable to instance {} that doesn't exist", instance_id);
                }
            }
            else
            {
                panic!("error: second argument to instance_add_variable() must be a string");
            }
        }
        else
        {
            panic!("error: first argument to instance_add_variable() must be a number");
        }
        return (Value::Number(0.0), false);
    }
    fn sim_func_instance_execute(&mut self, global : &mut GlobalState, mut args : Vec<Value>, isexpr : bool) -> (Value, bool)
    {
        if args.len() < 2
        {
            panic!("error: wrong number of arguments to instance_execute(); expected 2 or more, got {}", args.len());
        }
        if let Ok(instance_id_f) = self.list_pop_number(&mut args)
        {
            let instance_id = instance_id_f.round() as usize;
            if let Ok(func) = self.list_pop_func(&mut args)
            {
                if func.internal
                {
                    panic!("error: unsupported: tried to use instance_execute() with an internal function");
                }
                if let Some(defdata) = func.userdefdata
                {
                    if let Some(_inst) = global.instances.get_mut(&instance_id)
                    {
                        self.jump_to_function(&defdata, args.into_iter().rev().collect(), isexpr, func.predefined);
                        self.top_frame.instancestack.push(instance_id);
                    }
                    else
                    {
                        panic!("error: tried to add variable to instance {} that doesn't exist", instance_id);
                    }
                }
                else
                {
                    panic!("internal error: funcval was non-internal but had no userdefdata");
                }
            }
            else
            {
                panic!("error: second argument to instance_execute() must be a function");
            }
        }
        else
        {
            panic!("error: first argument to instance_execute() must be a number");
        }
        return (Value::Number(0.0), true);
    }
    fn sim_func_parse_text(&mut self, global : &mut GlobalState, mut args : Vec<Value>, _ : bool) -> (Value, bool)
    {
        if args.len() != 1
        {
            panic!("error: wrong number of arguments to parse_text(); expected 1, got {}", args.len());
        }
        if let Ok(text) = self.list_pop_text(&mut args)
        {
            let tokens = global.parser.tokenize(text, true);
            if let Some(ref ast) = global.parser.parse_program(&tokens, true)
            {
                return (ast_to_dict(ast), false);
            }
            else
            {
                panic!("error: string failed to parse");
            }
        }
        else
        {
            panic!("error: first argument to parse_text() must be a string");
        }
    }
    fn sim_func_compile_text(&mut self, global : &mut GlobalState, mut args : Vec<Value>, _ : bool) -> (Value, bool)
    {
        if args.len() != 1
        {
            panic!("error: wrong number of arguments to compile_text(); expected 1, got {}", args.len());
        }
        if let Ok(text) = self.list_pop_text(&mut args)
        {
            let tokens = global.parser.tokenize(text, true);
            if let Some(ref ast) = global.parser.parse_program(&tokens, true)
            {
                let code = compile_bytecode(ast);
                
                return (Value::Func(FuncVal
                    { internal : false, internalname : None, predefined : None,
                      userdefdata : Some(FuncSpec { varnames : Vec::new(), code : Rc::new(code), startaddr : 0, fromobj : false, parentobj : 0, forcecontext : 0 } )
                    } ), false);
            }
            else
            {
                panic!("error: string failed to parse");
            }
        }
        else
        {
            panic!("error: first argument to compile_text() must be a string");
        }
    }
    fn sim_func_compile_ast(&mut self, _global : &mut GlobalState, mut args : Vec<Value>, _ : bool) -> (Value, bool)
    {
        if args.len() != 1
        {
            panic!("error: wrong number of arguments to compile_ast(); expected 1, got {}", args.len());
        }
        if let Ok(dict) = self.list_pop_dict(&mut args)
        {
            let ast = dict_to_ast(&dict);
            
            let code = compile_bytecode(&ast);
            
            return (Value::Func(FuncVal
                { internal : false, internalname : None, predefined : None,
                  userdefdata : Some(FuncSpec { varnames : Vec::new(), code : Rc::new(code), startaddr : 0, fromobj : false, parentobj : 0, forcecontext : 0 } )
                } ), false);
        }
        else
        {
            panic!("error: first argument to compile_ast() must be a dictionary");
        }
    }
    fn internal_function_is_noreturn(&mut self, name : &String) -> bool
    {
        match name.as_str()
        {
            "instance_execute" => true,
            _ => false
        }
    }
    fn get_internal_function(&mut self, name : &String) -> Option<Box<Fn(&mut Interpreter, &mut GlobalState, Vec<Value>, bool) -> (Value, bool)>>
    {
        macro_rules! enbox {
            ( $x:ident ) =>
            {
                Some(Box::new(Interpreter::$x))
            }
        }
        match name.as_str()
        {
            "print" => enbox!(sim_func_print),
            "len" => enbox!(sim_func_len),
            "keys" => enbox!(sim_func_keys),
            "parse_text" => enbox!(sim_func_parse_text),
            "compile_text" => enbox!(sim_func_compile_text),
            "compile_ast" => enbox!(sim_func_compile_ast),
            "instance_execute" => enbox!(sim_func_instance_execute),
            "instance_create" => enbox!(sim_func_instance_create),
            "instance_add_variable" => enbox!(sim_func_instance_add_variable),
            _ => None
        }
    }
    fn get_opfunc(&mut self, op : u8) -> Option<Box<Fn(&mut Interpreter, &mut GlobalState)>>
    {
        macro_rules! enbox {
            ( $x:ident ) =>
            {
                Some(Box::new(Interpreter::$x))
            }
        }
        match op
        {
            NOP => enbox!(sim_NOP),
            PUSHFLT => enbox!(sim_PUSHFLT),
            PUSHSHORT => enbox!(sim_PUSHSHORT),
            PUSHSTR => enbox!(sim_PUSHSTR),
            PUSHNAME => enbox!(sim_PUSHNAME),
            PUSHVAR => enbox!(sim_PUSHVAR),
            DECLVAR => enbox!(sim_DECLVAR),
            DECLFAR => enbox!(sim_DECLFAR),
            BINSTATE => enbox!(sim_BINSTATE),
            BINOP => enbox!(sim_BINOP),
            UNOP => enbox!(sim_UNOP),
            INDIRECTION => enbox!(sim_INDIRECTION),
            EVALUATION => enbox!(sim_EVALUATION),
            FUNCCALL => enbox!(sim_FUNCCALL),
            FUNCEXPR => enbox!(sim_FUNCEXPR),
            FUNCDEF => enbox!(sim_FUNCDEF),
            LAMBDA => enbox!(sim_LAMBDA),
            OBJDEF => enbox!(sim_OBJDEF),
            COLLECTARRAY => enbox!(sim_COLLECTARRAY),
            COLLECTDICT => enbox!(sim_COLLECTDICT),
            ARRAYEXPR => enbox!(sim_ARRAYEXPR),
            BREAK => enbox!(sim_BREAK),
            CONTINUE => enbox!(sim_CONTINUE),
            IF => enbox!(sim_IF),
            IFELSE => enbox!(sim_IFELSE),
            WHILE => enbox!(sim_WHILE),
            FOR => enbox!(sim_FOR),
            SCOPE => enbox!(sim_SCOPE),
            UNSCOPE => enbox!(sim_UNSCOPE),
            WITH => enbox!(sim_WITH),
            EXIT => enbox!(sim_EXIT),
            RETURN => enbox!(sim_RETURN),
            LINENUM => enbox!(sim_LINENUM),
            _ => None
        }
    }
    fn read_string(&mut self) -> String
    {
        let code = self.get_code();
        if self.get_pc() >= code.len()
        {
            return "".to_string();
        }
        
        let mut bytes = Vec::<u8>::new();
        
        let mut c = self.pull_single_from_code();
        while c != 0 && self.get_pc() < code.len() // FIXME check if this should be < or <= (will only affect malformed bytecode, but still)
        {
            bytes.push(c);
            c = self.pull_single_from_code();
        }
        
        if let Ok(res) = std::str::from_utf8(&bytes)
        {
            return res.to_string();
        }
        else
        {
            return "".to_string();
        }
    }
    fn read_function(&mut self) -> (String, FuncSpec)
    {
        let code = self.get_code();
        
        let name = self.read_string();
        
        let argcount = unpack_u16(&self.pull_from_code(2));
        
        let bodylen = unpack_u64(&self.pull_from_code(8)) as usize;
        
        let mut args = Vec::<String>::new();
        for _ in 0..argcount
        {
            args.push(self.read_string());
        }
        
        let startaddr = self.get_pc();
        self.add_pc(bodylen);
        
        return (name, FuncSpec { varnames : args, code : Rc::clone(&code), startaddr, fromobj : false, parentobj : 0, forcecontext : 0 } );
    }
    
    fn read_lambda(&mut self) -> (HashMap<String, Value>, FuncSpec)
    {
        let code = self.get_code();
        
        let capturecount = unpack_u16(&self.pull_from_code(2)) as usize;
        
        if self.top_frame.stack.len() < capturecount*2
        {
            panic!("internal error: not enough values on stack to satisfy requirements of read_lambda (need {}, have {})", capturecount*2, self.top_frame.stack.len());
        }
        
        let mut captures = HashMap::<String, Value>::new();
        for _i in 0..capturecount
        {
            if let Ok(val) = self.stack_pop_any()
            {
                if let Ok(name) = self.stack_pop_text()
                {
                    if captures.contains_key(&name)
                    {
                        panic!("error: duplicate capture variable name `{}` in lambda capture expression", name);
                    }
                    captures.insert(name, val);
                }
                else
                {
                    panic!("internal error: read_lambda failed to collect capture name from stack");
                }
            }
            else
            {
                panic!("internal error: read_lambda failed to collect capture value from stack");
            }
        }
        
        let argcount = unpack_u16(&self.pull_from_code(2));
        
        let bodylen = unpack_u64(&self.pull_from_code(8)) as usize;
        
        let mut args = Vec::<String>::new();
        for _ in 0..argcount
        {
            args.push(self.read_string());
        }
        
        let startaddr = self.get_pc();
        self.add_pc(bodylen);
        
        return (captures, FuncSpec { varnames : args, code : Rc::clone(&code), startaddr, fromobj : false, parentobj : 0, forcecontext : 0 } );
    }
    #[allow(non_snake_case)] 
    fn sim_NOP(&mut self, _global : &mut GlobalState)
    {
        
    }
    #[allow(non_snake_case)]
    fn sim_PUSHFLT(&mut self, _global : &mut GlobalState)
    {
        let value = unpack_f64(&self.pull_from_code(8));
        self.top_frame.stack.push(Value::Number(value));
    }
    #[allow(non_snake_case)]
    fn sim_PUSHSHORT(&mut self, _global : &mut GlobalState)
    {
        let value = unpack_u16(&self.pull_from_code(2));
        self.top_frame.stack.push(Value::Number(value as f64));
    }
    #[allow(non_snake_case)]
    fn sim_PUSHSTR(&mut self, _global : &mut GlobalState)
    {
        let text = self.read_string();
        self.top_frame.stack.push(Value::Text(text));
    }
    #[allow(non_snake_case)]
    fn sim_PUSHNAME(&mut self, _global : &mut GlobalState)
    {
        let text = self.read_string();
        self.top_frame.stack.push(Value::Var(Variable::Direct(DirectVar{name:text})));
    }
    #[allow(non_snake_case)]
    fn sim_PUSHVAR(&mut self, global : &mut GlobalState)
    {
        let name = self.read_string();
        let dirvar = Variable::Direct(DirectVar{name : name.clone()}); // FIXME suboptimal but helps error message
        if let Some(val) = self.evaluate_or_store(global, &dirvar, None)
        {
            self.top_frame.stack.push(val);
        }
        else
        {
            panic!("error: tried to evaluate non-extant variable `{}`", name);
        }
    }
    
    fn stack_pop_number(&mut self) -> Result<f64, i32>
    {
        list_pop_generic!(self.top_frame.stack, Number)
    }
    fn stack_pop_text(&mut self) -> Result<String, i32>
    {
        list_pop_generic!(self.top_frame.stack, Text)
    }
    /*
    fn stack_pop_var(&mut self) -> Result<Variable, i32>
    {
        list_pop_generic!(self.top_frame.stack, Var)
    }
    */
    fn stack_pop_name(&mut self) -> Result<String, i32>
    {
        let var = list_pop_generic!(self.top_frame.stack, Var)?;
        if let Variable::Direct(DirectVar{name:text}) = var
        {
            Ok(text)
        }
        else
        {
            Err(1)
        }
    }
    fn stack_pop_any(&mut self) -> Result<Value, i32>
    {
        if let Some(val) = self.top_frame.stack.pop()
        {
            Ok(val)
        }
        else
        {
            Err(0)
        }
    }
    
    #[allow(non_snake_case)]
    fn sim_DECLVAR(&mut self, _global : &mut GlobalState)
    {
        if self.top_frame.stack.len() < 1
        {
            panic!("internal error: DECLVAR instruction requires 1 values on the stack but only found 0");
        }
        if let Ok(name) = self.stack_pop_name()
        {
            if let Some(scope) = self.top_frame.scopes.last_mut()
            {
                if scope.contains_key(&name)
                {
                    panic!("error: redeclared identifier {}", name);
                }
                scope.insert(name, Value::Number(0.0));
            }
            else
            {
                panic!("internal error: there are no scopes in the top frame");
            }
        }
        else
        {
            panic!("internal error: tried to declare a variable with a name of invalid type");
        }
    }
    
    #[allow(non_snake_case)]
    fn sim_DECLFAR(&mut self, global : &mut GlobalState)
    {
        if self.top_frame.stack.len() < 1
        {
            panic!("internal error: DECLFAR instruction requires 1 values on the stack but only found 0");
        }
        if let Ok(name) = self.stack_pop_name()
        {
            if let Some(instance_id) = self.top_frame.instancestack.last()
            {
                if let Some(instance) = global.instances.get_mut(instance_id)
                {
                    if !instance.variables.contains_key(&name)
                    {
                        instance.variables.insert(name, Value::Number(0.0));
                    }
                    else
                    {
                        panic!("error: redeclared identifier {}", name);
                    }
                }
                else
                {
                    panic!("error: tried to declare instance variable but instance of current scope ({}) no longer exists", instance_id);
                }
            }
            else
            {
                panic!("error: tried to declare instance variable when not executing within instance scope");
            }
        }
        else
        {
            panic!("internal error: tried to declare instance variable with non-var-name type name");
        }
    }
    /*
    def sim_DECLFAR():
        if string in instances[instancestack[-1]].variables:
            print(f"error: redeclared identifier {string}")
            exit()
        instances[instancestack[-1]].variables[string] = 0
    */
    
    #[allow(non_snake_case)]
    fn sim_INDIRECTION(&mut self, global : &mut GlobalState)
    {
        if self.top_frame.stack.len() < 2
        {
            panic!("internal error: INDIRECTION instruction requires 2 values on the stack but only found {}", self.top_frame.stack.len());
        }
        if let Ok(right) = self.stack_pop_name()
        {
            if let Ok(left) = self.stack_pop_number()
            {
                let id = left.round() as usize;
                
                if global.instances.contains_key(&id)
                {
                    self.top_frame.stack.push(Value::Var(Variable::Indirect(IndirectVar{ident : id, name : right})));
                }
                else
                {
                    panic!("error: tried to perform indirection on instance {} that doesn't exist", id)
                }
            }
            else
            {
                panic!("error: tried to use indirection on a type that can't be an identifier")
            }
        }
        else
        {
            panic!("error: FIXME ADFGJAWEIFASDFJGERG")
        }
    }
    #[allow(non_snake_case)]
    fn sim_EVALUATION(&mut self, global : &mut GlobalState)
    {
        if let Some(val) = self.top_frame.stack.pop()
        {
            match val
            {
                Value::Var(var) =>
                {
                    match var
                    {
                        Variable::Indirect(_) |
                        Variable::Array(_) =>
                        {
                            if let Some(value) = self.evaluate_or_store(global, &var, None)
                            {
                                self.top_frame.stack.push(value);
                            }
                            else
                            {
                                panic!("internal error: evaluate_or_store returned None when just storing a variable");
                            }
                        }
                        Variable::Direct(_) =>
                        {
                            panic!("internal error: tried to evaluate direct variable `{}`\n(note: the evaluation instruction is for indirect (id.y) variables and array (arr[0]) variables; bytecode metaprogramming for dynamic direct variable access is unsupported)");
                        }
                    }
                }
                etc =>
                {
                    panic!("internal error: tried to evaluate non-variable value\n({})", format_val(&etc).unwrap());
                }
            }
        }
        else
        {
            panic!("internal error: EVALUATION instruction requires 1 values on the stack but only found 0");
        }
    }
    fn call_function(&mut self, global : &mut GlobalState, funcdata : FuncVal, args : Vec<Value>, isexpr : bool)
    {
        if funcdata.internal
        {
            if let Some(name) = funcdata.internalname
            {
                if let Some(internal_func) = self.get_internal_function(&name)
                {
                    let (ret, moved_frame) = internal_func(self, global, args, isexpr);
                    if isexpr && !self.internal_function_is_noreturn(&name)
                    {
                        let frames_len = self.frames.len(); // for the panic down there (non-lexical borrow lifetimes pls happen soon)
                        if !moved_frame
                        {
                            self.top_frame.stack.push(ret);
                        }
                        else if let Some(frame) = self.frames.last_mut()
                        {
                            frame.stack.push(ret);
                        }
                        else
                        {
                            panic!("internal error: couldn't find old frame after calling function `{}` that moves the frame; framestack has length {}", name, frames_len);
                        }
                    }
                }
                else
                {
                    panic!("internal error: tried to look up non-extant internal function after it was already referenced in a value (this should be unreachable!)");
                }
            }
            else
            {
                panic!("internal error: function variable describing internal function is lacking its function name");
            }
        }
        else if let Some(defdata) = funcdata.userdefdata
        {
            if !defdata.fromobj
            {
                self.jump_to_function(&defdata, args, isexpr, funcdata.predefined);
            }
            else if defdata.forcecontext != 0
            {
                if let Some(inst) = global.instances.get(&defdata.forcecontext)
                {
                    // FIXME ?
                    if !global.objects.contains_key(&inst.objtype)
                    {
                        panic!("error: tried to access data from object type {} that no longer exists", inst.objtype);
                    }
                    if defdata.parentobj != inst.objtype
                    {
                        panic!("error: tried to call function from object type {} in the context of an instance of object type {}", defdata.parentobj, inst.objtype);
                    }
                    self.jump_to_function(&defdata, args, isexpr, funcdata.predefined);
                    self.top_frame.instancestack.push(defdata.forcecontext);
                }
            }
            else
            {
                // FIXME ?
                let inst_copy : Vec<usize> = self.top_frame.instancestack.iter().cloned().rev().collect();
                for instance in inst_copy
                {
                    if let Some(inst) = global.instances.get(&instance)
                    {
                        if !global.objects.contains_key(&inst.objtype)
                        {
                            panic!("error: tried to access data from object type {} that no longer exists", inst.objtype);
                        }
                        if defdata.parentobj != inst.objtype
                        {
                            panic!("error: tried to call function from object type {} in the context of an instance of object type {}", defdata.parentobj, inst.objtype);
                        }
                        self.jump_to_function(&defdata, args, isexpr, funcdata.predefined);
                        self.top_frame.instancestack.push(instance);
                        return;
                    }
                    else
                    {
                        panic!("TODO error aidsfgojaedfouajiefjfbdgnwru");
                    }
                }
            }
        }
        else
        {
            panic!("TODO error dfghjdftjsdrfhsrtj");
        }
    }
    
    #[allow(non_snake_case)]
    fn handle_func_call_or_expr(&mut self, global : &mut GlobalState, isexpr : bool)
    {
        if let Some(funcdata) = self.top_frame.stack.pop()
        {
            if let Some(argcount_val) = self.top_frame.stack.pop()
            {
                if let Value::Number(argcount) = argcount_val
                {
                    let mut args = Vec::<Value>::new();
                    for _i in 0..(argcount.round() as usize)
                    {
                        if let Some(arg) = self.top_frame.stack.pop()
                        {
                            args.push(arg);
                        }
                        else
                        {
                            panic!("internal error: fewer variables on stack than expected in FUNCEXPR");
                        }
                    }
                    if let Value::Var(var) = funcdata
                    {
                        if let Some(funcdata_val) = self.evaluate_or_store(global, &var, None)
                        {
                            if let Value::Func(funcdata) = funcdata_val
                            {
                                self.call_function(global, funcdata, args, isexpr)
                            }
                            else
                            {
                                panic!("internal error: variable meant to hold function data in FUNCEXPR was not holding function data");
                            }
                        }
                        else
                        {
                            panic!("internal error: variable meant to hold function data in FUNCEXPR was invalid");
                        }
                    }
                    else if let Value::Func(funcdata) = funcdata
                    {
                        self.call_function(global, funcdata, args, isexpr)
                    }
                    else
                    {
                        panic!("internal error: variable meant to hold function data in FUNCEXPR was not holding function data");
                    }
                }
                else
                {
                    panic!("internal error: number on stack of arguments to function was not a number");
                }
            }
            else
            {
                panic!("internal error: not enough values on stack to run instruction FUNCEXPR");
            }
        }
        else
        {
            panic!("internal error: not enough values on stack to run instruction FUNCEXPR");
        }
    }
    #[allow(non_snake_case)]
    fn sim_FUNCCALL(&mut self, global : &mut GlobalState)
    {
        self.handle_func_call_or_expr(global, false);
    }
    #[allow(non_snake_case)]
    fn sim_FUNCEXPR(&mut self, global : &mut GlobalState)
    {
        self.handle_func_call_or_expr(global, true);
    }
    
    #[allow(non_snake_case)]
    fn sim_SCOPE(&mut self, _global : &mut GlobalState)
    {
        self.top_frame.scopes.push(HashMap::new());
        if self.top_frame.scopes.len() >= 0x10000
        {
            panic!("error: scope recursion limit of 0x10000 reached at line {}\n(note: use more functions!)", self.top_frame.currline);
        }
    }
    #[allow(non_snake_case)]
    fn sim_UNSCOPE(&mut self, _global : &mut GlobalState)
    {
        let immediate = unpack_u16(&self.pull_from_code(2)) as usize;
        
        while self.top_frame.scopes.len() > immediate+1
        {
            self.top_frame.scopes.pop();
        }
    }
    
    fn pop_controlstack_until_loop(&mut self)
    {
        let mut foundloop = false;
        
        if let Some(controller) = self.top_frame.controlstack.last()
        {
            if controller.controltype == WHILE || controller.controltype == FOR // TODO: add WITH
            {
                foundloop = true;
            }
        }
        
        if !foundloop
        {
            self.top_frame.controlstack.pop();
            self.pop_controlstack_until_loop();
        }
    }
    
    #[allow(non_snake_case)]
    fn sim_BREAK(&mut self, _global : &mut GlobalState)
    {
        self.pop_controlstack_until_loop();
        
        if self.top_frame.controlstack.len() == 0
        {
            panic!("error: break instruction not inside of loop");
        }
        
        let controller = self.top_frame.controlstack.last().unwrap().clone();
        
        if controller.controltype == WHILE
        {
            self.set_pc(controller.controlpoints[2]);
            self.drain_scopes(controller.scopes);
            self.top_frame.controlstack.pop();
        }
        else if controller.controltype == FOR
        {
            self.set_pc(controller.controlpoints[3]);
            self.drain_scopes(controller.scopes);
            self.top_frame.controlstack.pop();
        }
        else
        {
            panic!("FIXME: unimplemented BREAK out from 0x{:02X} loop", controller.controltype);
        }
    }
    #[allow(non_snake_case)]
    fn sim_CONTINUE(&mut self, _global : &mut GlobalState)
    {
        self.pop_controlstack_until_loop();
        
        if self.top_frame.controlstack.len() == 0
        {
            panic!("error: continue instruction not inside of loop");
        }
        
        let controller = self.top_frame.controlstack.last().unwrap().clone();
        
        if controller.controltype == WHILE
        {
            self.set_pc(controller.controlpoints[0]);
            self.drain_scopes(controller.scopes);
        }
        else if controller.controltype == FOR
        {
            self.set_pc(controller.controlpoints[1]);
            self.suppress_for_expr_end = true;
            self.drain_scopes(controller.scopes);
        }
        else
        {
            panic!("FIXME: unimplemented CONTINUE out from 0x{:02X} loop", controller.controltype);
        }
    }
    #[allow(non_snake_case)]
    fn sim_IF(&mut self, _global : &mut GlobalState)
    {
        let exprlen = unpack_u64(&self.pull_from_code(8)) as usize;
        let codelen = unpack_u64(&self.pull_from_code(8)) as usize;
        let current_pc = self.get_pc();
        let scopelen = self.top_frame.scopes.len() as u16;
        self.top_frame.controlstack.push(ControlData{controltype : IF, controlpoints : vec!(current_pc+exprlen, current_pc+exprlen+codelen), scopes : scopelen, other : None});
    }
    #[allow(non_snake_case)]
    fn sim_IFELSE(&mut self, _global : &mut GlobalState)
    {
        let exprlen = unpack_u64(&self.pull_from_code(8)) as usize;
        let codelen1 = unpack_u64(&self.pull_from_code(8)) as usize;
        let codelen2 = unpack_u64(&self.pull_from_code(8)) as usize;
        let current_pc = self.get_pc();
        let scopelen = self.top_frame.scopes.len() as u16;
        self.top_frame.controlstack.push(ControlData{controltype : IFELSE, controlpoints : vec!(current_pc+exprlen, current_pc+exprlen+codelen1, current_pc+exprlen+codelen1+codelen2), scopes : scopelen, other : None});
    }
    #[allow(non_snake_case)]
    fn sim_WHILE(&mut self, _global : &mut GlobalState)
    {
        let exprlen = unpack_u64(&self.pull_from_code(8)) as usize;
        let codelen = unpack_u64(&self.pull_from_code(8)) as usize;
        let current_pc = self.get_pc();
        let scopelen = self.top_frame.scopes.len() as u16;
        self.top_frame.controlstack.push(ControlData{controltype : WHILE, controlpoints : vec!(current_pc, current_pc+exprlen, current_pc+exprlen+codelen), scopes : scopelen, other : None});
    }
    #[allow(non_snake_case)]
    fn sim_FOR(&mut self, _global : &mut GlobalState)
    {
        let exprlen = unpack_u64(&self.pull_from_code(8)) as usize;
        let postlen = unpack_u64(&self.pull_from_code(8)) as usize;
        let codelen = unpack_u64(&self.pull_from_code(8)) as usize;
        let current_pc = self.get_pc();
        let scopelen = self.top_frame.scopes.len() as u16;
        self.top_frame.controlstack.push(ControlData{controltype : FOR, controlpoints : vec!(current_pc, current_pc+exprlen, current_pc+exprlen+postlen, current_pc+exprlen+postlen+codelen), scopes : scopelen, other : None});
    }
    #[allow(non_snake_case)]
    fn sim_WITH(&mut self, global : &mut GlobalState)
    {
        if self.top_frame.stack.len() < 1
        {
            panic!("internal error: WITH instruction requires 1 values on the stack but found 0");
        }
        // NOTE: for with(), the self.top_frame.scopes.len() >= 0xFFFF error case is handled by SCOPE instruction
        if let Ok(expr) = self.stack_pop_number()
        {
            let other_id = expr.round() as usize;
            
            let codelen = unpack_u64(&self.pull_from_code(8));
            
            let current_pc = self.get_pc();
            
            if global.instances.contains_key(&other_id)
            {
                self.top_frame.instancestack.push(other_id);
                
                self.top_frame.controlstack.push(ControlData{controltype : WITH, controlpoints : vec!(current_pc, current_pc + codelen as usize), scopes : self.top_frame.scopes.len() as u16, other : Some(VecDeque::new())});
            }
            else if let Some(instance_id_list) = global.instances_by_type.get(&other_id)
            {
                if let Some(first) = instance_id_list.first()
                {
                    self.top_frame.instancestack.push(*first);
                    let mut copylist : VecDeque<usize> = instance_id_list.iter().cloned().collect();
                    copylist.pop_front();
                    self.top_frame.controlstack.push(ControlData{controltype : WITH, controlpoints : vec!(current_pc, current_pc + codelen as usize), scopes : self.top_frame.scopes.len() as u16, other : Some(copylist)});
                }
                else
                {
                    // silently skip block if there are no instances of this object type
                    self.add_pc(codelen as usize);
                }
            }
            else
            {
                panic!("error: tried to use non-existant instance in with expression");
            }
        }
        else
        {
            panic!("error: tried to use with() on a non-numeric expression (instance ids and object ids are numeric)");
        }
    }
    #[allow(non_snake_case)]
    fn sim_FUNCDEF(&mut self, _global : &mut GlobalState)
    {
        let (funcname, myfuncspec) = self.read_function();
        if let Some(scope) = self.top_frame.scopes.last_mut()
        {
            if scope.contains_key(&funcname)
            {
                panic!("error: redeclared identifier {}, name")
            }
            scope.insert(funcname, Value::Func(FuncVal { internal : false, internalname : None, predefined : None, userdefdata : Some(myfuncspec) }));
        }
        else
        {
            panic!("internal error: there are no scopes in the top frame");
        }
    }
    
    #[allow(non_snake_case)]
    fn sim_BINSTATE(&mut self, global : &mut GlobalState)
    {
        if self.top_frame.stack.len() < 2
        {
            panic!("internal error: BINSTATE instruction requires 2 values on the stack but found {}", self.top_frame.stack.len());
        }
        
        let immediate = self.pull_single_from_code();
        
        if let Some(value) = self.top_frame.stack.pop()
        {
            if let Some(var_val) = self.top_frame.stack.pop()
            {
                if let Value::Var(var) = var_val
                {
                    if immediate == 0x00
                    {
                        self.evaluate_or_store(global, &var, Some(value));
                    }
                    else if let Some(opfunc) = get_binop_function(immediate)
                    {
                        if let Some(var_initial_value) = self.evaluate_or_store(global, &var, None)
                        {
                            match opfunc(&var_initial_value, &value)
                            {
                                Ok(var_new_value) =>
                                {
                                    self.evaluate_or_store(global, &var, Some(var_new_value));
                                }
                                Err(text) =>
                                {
                                    //panic!("error: disallowed binary statement\n({})\n(line {})", text, self.top_frame.currline);
                                    panic!("error: disallowed binary statement\n({})", text);
                                }
                            }
                        }
                        else
                        {
                            panic!("internal error: evaluate_or_store returned None when just accessing value");
                        }
                    }
                    else
                    {
                        panic!("internal error: unknown binary operation 0x{:02X}", immediate);
                    }
                }
                else
                {
                    panic!("primary argument to BINSTATE was not a variable");
                }
            }
            else
            {
                panic!("internal error: not enough values on stack to run instruction BINSTATE (this error should be inaccessible)");
            }
        }
        else
        {
            panic!("internal error: not enough values on stack to run instruction BINSTATE (this error should be inaccessible)");
        }
    }
    
    #[allow(non_snake_case)]
    fn sim_BINOP(&mut self, _global : &mut GlobalState)
    {
        if self.top_frame.stack.len() < 2
        {
            panic!("internal error: BINOP instruction requires 2 values on the stack but found {}", self.top_frame.stack.len());
        }
        
        let immediate = self.pull_single_from_code();
        
        if let Some(right) = self.top_frame.stack.pop()
        {
            if let Some(left) = self.top_frame.stack.pop()
            {
                if let Some(opfunc) = get_binop_function(immediate)
                {
                    match opfunc(&left, &right)
                    {
                        Ok(new_value) =>
                        {
                            self.top_frame.stack.push(new_value);
                        }
                        Err(text) =>
                        {
                            panic!("error: disallowed binary expression\n({})\n(value 1: {})\n(value 2: {})", text, format_val(&left).unwrap(), format_val(&right).unwrap());
                        }
                    }
                }
                else
                {
                    panic!("internal error: unknown binary operation 0x{:02X}", immediate);
                }
            }
            else
            {
                panic!("internal error: not enough values on stack to run instruction BINOP (this error should be inaccessible!)");
            }
        }
        else
        {
            panic!("internal error: not enough values on stack to run instruction BINOP (this error should be inaccessible!)");
        }
    }
    
    #[allow(non_snake_case)]
    fn sim_UNOP(&mut self, _global : &mut GlobalState)
    {
        if self.top_frame.stack.len() < 1
        {
            panic!("internal error: UNOP instruction requires 2 values on the stack but found {}", self.top_frame.stack.len());
        }
        
        let immediate = self.pull_single_from_code();
        
        if let Some(value) = self.top_frame.stack.pop()
        {
            if let Some(opfunc) = get_unop_function(immediate)
            {
                match opfunc(&value)
                {
                    Ok(new_value) =>
                    {
                        self.top_frame.stack.push(new_value);
                    }
                    Err(text) =>
                    {
                        panic!("error: disallowed unary expression\n({})", text);
                    }
                }
            }
            else
            {
                panic!("internal error: unknown binary operation 0x{:02X}", immediate);
            }
        }
        else
        {
            panic!("internal error: not enough values on stack to run instruction UNOP (this error should be inaccessible!)");
        }
    }
    #[allow(non_snake_case)]
    fn sim_LAMBDA(&mut self, _global : &mut GlobalState)
    {
        let (captures, myfuncspec) = self.read_lambda();
        self.top_frame.stack.push(Value::Func(FuncVal{internal : false, internalname : None, predefined : Some(captures), userdefdata : Some(myfuncspec)}));
    }
    #[allow(non_snake_case)]
    fn sim_OBJDEF(&mut self, global : &mut GlobalState)
    {
        let name = self.read_string();
        if global.objectnames.contains_key(&name)
        {
            panic!("error: redeclared object {}", name);
        }
        
        let object_id = global.object_id;
        let numfuncs = unpack_u16(&self.pull_from_code(2));
        
        let mut funcs = HashMap::<String, FuncSpec>::new();
        for _ in 0..numfuncs
        {
            let (funcname, mut myfuncspec) = self.read_function();
            myfuncspec.fromobj = true;
            myfuncspec.parentobj = object_id;
            if funcs.contains_key(&funcname)
            {
                panic!("error: redeclared function {} in object {}", funcname, name);
            }
            funcs.insert(funcname, myfuncspec);
        }
        
        global.objectnames.insert(name.clone(), object_id);
        global.objects.insert(object_id, ObjSpec { ident : object_id, name, functions : funcs });
        global.instances_by_type.insert(object_id, Vec::new());
        
        global.object_id += 1;
    }
    #[allow(non_snake_case)]
    fn sim_COLLECTARRAY(&mut self, _global : &mut GlobalState)
    {
        let numvals = unpack_u16(&self.pull_from_code(2)) as usize;
        if self.top_frame.stack.len() < numvals
        {
            panic!("internal error: not enough values on stack for COLLECTARRAY instruction to build array (need {}, have {})", numvals, self.top_frame.stack.len());
        }
        let mut myarray = VecDeque::<Value>::new();
        for _i in 0..numvals
        {
            if let Ok(val) = self.stack_pop_any()
            {
                myarray.push_front(val);
            }
            else
            {
                panic!("internal error: COLLECTARRAY instruction failed to collect values from stack (this error should be unreachable!)");
            }
        }
        self.top_frame.stack.push(Value::Array(myarray));
    }
    #[allow(non_snake_case)]
    fn sim_COLLECTDICT(&mut self, _global : &mut GlobalState)
    {
        let numvals = unpack_u16(&self.pull_from_code(2)) as usize;
        if self.top_frame.stack.len() < numvals*2
        {
            panic!("internal error: not enough values on stack for COLLECTDICT instruction to build dict (need {}, have {})", numvals*2, self.top_frame.stack.len());
        }
        
        let mut names = VecDeque::<HashableValue>::new();
        let mut values = VecDeque::<Value>::new();
        
        for _i in 0..numvals
        {
            if let Ok(val) = self.stack_pop_any()
            {
                if let Ok(key) = self.stack_pop_any()
                {
                    values.push_front(val);
                    match key
                    {
                        Value::Number(number) =>
                        {
                            names.push_front(HashableValue::Number(number));
                        }
                        Value::Text(text) =>
                        {
                            names.push_front(HashableValue::Text(text));
                        }
                        _ =>
                        {
                            panic!("error: dictionary key must be a string or number; was {:?}; line {}", key, self.top_frame.currline);
                        }
                    }
                }
                else
                {
                    panic!("internal error: COLLECTDICT instruction failed to collect values from stack");
                }
            }
            else
            {
                panic!("internal error: COLLECTDICT instruction failed to collect values from stack");
            }
        }
        let mut mydict = HashMap::<HashableValue, Value>::new();
        for (name, value) in names.into_iter().zip(values.into_iter())
        {
            mydict.insert(name, value);
        }
        self.top_frame.stack.push(Value::Dict(mydict));
    }
    #[allow(non_snake_case)]
    fn sim_ARRAYEXPR(&mut self, _global : &mut GlobalState)
    {
        if self.top_frame.stack.len() < 2
        {
            panic!("internal error: ARRAYEXPR instruction requires 2 values on the stack but found {}", self.top_frame.stack.len());
        }
        if let Ok(index) = self.stack_pop_any()
        {
            if let Ok(array) = self.stack_pop_any()
            {
                match array
                {
                    Value::Var(Variable::Array(mut arrayvar)) =>
                    {
                        arrayvar.indexes.push_back(index);
                        self.top_frame.stack.push(Value::Var(Variable::Array(arrayvar)));
                    }
                    Value::Var(Variable::Direct(mut dirvar)) =>
                    {
                        self.top_frame.stack.push(Value::Var(Variable::Array(ArrayVar { location : NonArrayVariable::Direct(dirvar), indexes : vec!(index).into_iter().collect() } )));
                    }
                    Value::Var(Variable::Indirect(mut indirvar)) =>
                    {
                        self.top_frame.stack.push(Value::Var(Variable::Array(ArrayVar { location : NonArrayVariable::Indirect(indirvar), indexes : vec!(index).into_iter().collect() } )));
                    }
                    Value::Array(mut array) =>
                    {
                        self.top_frame.stack.push(Value::Var(Variable::Array(ArrayVar { location : NonArrayVariable::ActualArray(array), indexes : vec!(index).into_iter().collect() } )));
                    }
                    _ =>
                    {
                        //panic!("error: tried to use array indexing on a non-indexable value\n{}", array);
                        panic!("error: tried to use array indexing on a non-indexable value");
                    }
                }
            }
            else
            {
                panic!("internal error: TODO write error askdgfauiowef");
            }
        }
        else
        {
            panic!("internal error: TODO write error askdgfauiowef");
        }
    }
    #[allow(non_snake_case)]
    fn sim_EXIT(&mut self, _global : &mut GlobalState) // an exit is a return with no value
    {
        if let Some(top_frame) = self.frames.pop()
        {
            let frame_was_expr = self.top_frame.isexpr;
            self.top_frame = top_frame;
            // exit implies no pushed variable. if the outside expects a value, push it
            if frame_was_expr
            {
                self.top_frame.stack.push(Value::Number(0.0));
            }
        }
        else
        {
            self.doexit = true;
        }
    }
    #[allow(non_snake_case)]
    fn sim_RETURN(&mut self, _global : &mut GlobalState)
    {
        if let Some(old_frame) = self.frames.pop()
        {
            let inner_frame_stack_last = self.top_frame.stack.pop();
            let frame_was_expr = self.top_frame.isexpr;
            self.top_frame = old_frame;
            // exit implies no pushed variable. if the outside expects a value, push it
            if frame_was_expr
            {
                if let Some(val) = inner_frame_stack_last
                {
                    self.top_frame.stack.push(val);
                }
                else
                {
                    panic!("error: RETURN instruction needed a value remaining on the inner frame's stack, but there were none");
                    //self.top_frame.stack.push(Value::Number(0.0));
                }
            }
        }
        else
        {
            panic!("error: attempted to return from global code; use exit() instead");
        }
    }
    #[allow(non_snake_case)]
    fn sim_LINENUM(&mut self, _global : &mut GlobalState)
    {
        self.top_frame.currline = unpack_u64(&self.pull_from_code(8)) as usize;
    }
    
    fn drain_scopes(&mut self, desired_depth : u16)
    {
        while self.top_frame.scopes.len() > desired_depth as usize
        {
            self.top_frame.scopes.pop();
        }
    }
    
    fn step(&mut self, global : &mut GlobalState) -> bool // TODO: return whether there was an error or not
    {
        let code = self.get_code();
        
        if self.top_frame.pc >= code.len()
        {
            println!("internal error: ran past end of code");
            return false;
        }
        let op = self.pull_single_from_code();
        
        if let Some(opfunc) = self.get_opfunc(op)
        {
            opfunc(self, global);
            
            if let Some(mut controller) = self.top_frame.controlstack.pop()
            {
                let mut must_put_back = true;
                if controller.controlpoints.contains(&self.get_pc())
                {
                    if controller.controltype == WHILE
                    {
                        // if we are at the end of the expression, test it, jump outside of the loop if it's false
                        if self.get_pc() == controller.controlpoints[1]
                        {
                            if let Ok(testval) = self.stack_pop_any()
                            {
                                if !value_truthy(&testval)
                                {
                                    self.set_pc(controller.controlpoints[2]);
                                    self.drain_scopes(controller.scopes);
                                    must_put_back = false;
                                }
                            }
                            else
                            {
                                panic!("internal error: not enough values on stack while handling WHILE controller");
                            }
                        }
                        // if we are at the end of the loop, go back to the expression
                        else if self.get_pc() == controller.controlpoints[2]
                        {
                            self.set_pc(controller.controlpoints[0]);
                            self.drain_scopes(controller.scopes);
                        }
                    }
                    else if controller.controltype == IFELSE
                    {
                        if self.get_pc() == controller.controlpoints[0]
                        {
                            // if we are at the end of the expression, test it, jump to the "else" block if it's false
                            if let Ok(testval) = self.stack_pop_any()
                            {
                                if !value_truthy(&testval)
                                {
                                    self.set_pc(controller.controlpoints[1]);
                                }
                            }
                            else
                            {
                                panic!("internal error: not enough values on stack while handling IFELSE controller");
                            }
                        }
                        else if self.get_pc() == controller.controlpoints[1]
                        {
                            // end of the main block, jump to the end of the "else" block
                            self.set_pc(controller.controlpoints[2]);
                            self.drain_scopes(controller.scopes);
                            must_put_back = false;
                        }
                        else if self.get_pc() == controller.controlpoints[2]
                        {
                            // end of the "else" block, clean up
                            self.drain_scopes(controller.scopes);
                            must_put_back = false;
                        }
                    }
                    else if controller.controltype == IF
                    {
                        if self.get_pc() == controller.controlpoints[0]
                        {
                            // if we are at the end of the expression, test it, jump past the block if it's false
                            if let Ok(testval) = self.stack_pop_any()
                            {
                                if !value_truthy(&testval)
                                {
                                    self.set_pc(controller.controlpoints[1]);
                                    self.drain_scopes(controller.scopes);
                                    must_put_back = false;
                                }
                            }
                            else
                            {
                                panic!("internal error: not enough values on stack while handling IF controller");
                            }
                        }
                    }
                    else if controller.controltype == FOR
                    {
                        if self.get_pc() == controller.controlpoints[1]
                        {
                            if self.suppress_for_expr_end
                            {
                                self.suppress_for_expr_end = false;
                            }
                            else
                            {
                                // if we are at the end of the loop expression, test it, jump past the block if it's false
                                if let Ok(testval) = self.stack_pop_any()
                                {
                                    if !value_truthy(&testval)
                                    {
                                        self.set_pc(controller.controlpoints[3]);
                                        self.drain_scopes(controller.scopes);
                                        must_put_back = false;
                                    }
                                    // otherwise jump to code (end of post expression)
                                    else
                                    {
                                        self.set_pc(controller.controlpoints[2]);
                                    }
                                }
                                else
                                {
                                    panic!("internal error: not enough values on stack while handling FOR controller");
                                }
                            }
                        }
                        else if self.get_pc() == controller.controlpoints[2]
                        {
                            // if we are at the end of the post expression, jump to the expression
                            self.set_pc(controller.controlpoints[0]);
                        }
                        else if self.get_pc() == controller.controlpoints[3]
                        {
                            // if we are at the end of the code block, jump to the post expression
                            self.set_pc(controller.controlpoints[1]);
                        }
                    }
                    else if controller.controltype == WITH
                    {
                        if self.get_pc() == controller.controlpoints[1]
                        {
                            if let Some(ref mut inst_list) = controller.other
                            {
                                if let Some(next_instance) = inst_list.remove(0)
                                {
                                    self.top_frame.instancestack.pop();
                                    self.top_frame.instancestack.push(next_instance);
                                    self.set_pc(controller.controlpoints[0]);
                                }
                                else
                                {
                                    self.top_frame.instancestack.pop();
                                    // FIXME do we have to drain scopes here or is it always consistent?
                                    must_put_back = false;
                                }
                            }
                        }
                    }
                    else
                    {
                        panic!("internal error: unknown controller type {:02X}", controller.controltype);
                    }
                }
                if must_put_back
                {
                    self.top_frame.controlstack.push(controller);
                }
            }
            return !self.doexit;
        }
        else
        {
            println!("internal error: unknown operation 0x{:02X}", op);
            println!("line: {}", self.top_frame.currline);
            return false;
        }
    }
    // if value is None, finds and returns appropriate value; otherwise, stores value and returns None
    fn evaluate_or_store(&mut self, global : &mut GlobalState, variable : &Variable, value : Option<Value>) -> Option<Value>
    {
        macro_rules! assign_or_return_indexed {
            ( $value:expr, $var:expr, $indexes:expr, $isconst:expr ) =>
            {
                unsafe
                {
                    let mut ptr = $var as *mut Value;
                    
                    let num_indexes = $indexes.len();
                    let mut current_index = 0;
                    
                    for index in &$indexes
                    {
                        if let Value::Array(ref mut newvar) = *ptr
                        {
                            if let Value::Number(indexnum) = index
                            {
                                if let Some(newvar2) = newvar.get_mut(indexnum.round() as usize)
                                {
                                    ptr = newvar2 as *mut Value;
                                }
                                else
                                {
                                    panic!("error: tried to access non-extant index {} of an array", indexnum);
                                }
                            }
                            else
                            {
                                panic!("error: tried to use a non-number as an array index");
                            }
                        }
                        else if let Value::Dict(ref mut newvar) = *ptr
                        {
                            if let Value::Number(indexnum) = index
                            {
                                if let Some(newvar2) = newvar.get_mut(&HashableValue::Number(*indexnum))
                                {
                                    ptr = newvar2 as *mut Value;
                                }
                                else
                                {
                                    panic!("error: tried to access non-extant index {} of a dict", indexnum);
                                }
                            }
                            else if let Value::Text(indexstr) = index
                            {
                                if let Some(newvar2) = newvar.get_mut(&HashableValue::Text(indexstr.clone()))
                                {
                                    ptr = newvar2 as *mut Value;
                                }
                                else
                                {
                                    panic!("error: tried to access non-extant index {} of a dict", indexstr);
                                }
                            }
                            else
                            {
                                panic!("error: tried to use a non-number, non-string as a dict index");
                            }
                        }
                        else if let Value::Text(ref mut text) = *ptr
                        {
                            if current_index+1 != num_indexes
                            {
                                // FIXME should we just treat further indexes as 0? that's what they would do if they were indexes into the substring at that index anyway, so...
                                panic!("error: tried to index into the value at another index in a string (i.e. tried to do something like \"asdf\"[0][0])");
                            }
                            else
                            {
                                if let Value::Number(indexnum) = index
                                {
                                    let mut realindex = ((indexnum.round() as i64) % text.len() as i64) as usize;
                                    
                                    
                                    if let Some(value) = $value
                                    {
                                        if let Value::Text(mychar) = value
                                        {
                                            if mychar.len() == 1
                                            {
                                                let mut codepoints = text.chars().collect::<Vec<char>>();
                                                codepoints[realindex] = mychar.chars().next().unwrap();
                                                /*
                                                // turn array of codepoints back into string
                                                */
                                                let newstr : String = codepoints.iter().collect();
                                                *ptr = Value::Text(newstr);
                                                return None;
                                            }
                                            else
                                            {
                                                panic!("error: tried to assign to an index into a string with a string that was not exactly one character long (was {} characters long)", mychar.len());
                                            }
                                        }
                                        else
                                        {
                                            panic!("error: tried to assign non-string to an index into a string (assigning by codepoint is not supported yet)");
                                        }
                                    }
                                    else
                                    {
                                        let mychar = text.chars().collect::<Vec<char>>()[realindex];
                                        let mut newstr = String::new();
                                        newstr.push(mychar);
                                        return Some(Value::Text(newstr));
                                    }
                                }
                                else
                                {
                                    panic!("error: tried to use a non-number as an index into a string");
                                }
                            }
                        }
                        else
                        {
                            panic!("error: tried to index into a non-array, non-dict value");
                        }
                        current_index += 1;
                    }
                    
                    if let Some(value) = $value
                    {
                        if $isconst
                        {
                            panic!("error: tried to assign to non-variable or read-only value");
                        }
                        else
                        {
                            *ptr = value.clone();
                        }
                        
                        return None;
                    }
                    else
                    {
                        return Some((*ptr).clone());
                    }
                }
            }
        }
        macro_rules! check_frame_dirvar_arrayed {
            ( $frame:expr, $dirvar:expr, $value:expr, $indexes:expr ) =>
            {
                // FIXME: do I even want to search up instance stacks rather than just accessing the main one?
                for scope in $frame.scopes.iter_mut().rev()
                {
                    if let Some(var) = scope.get_mut(&$dirvar.name)
                    {
                        assign_or_return_indexed!($value, var, $indexes, false);
                    }
                }
                for id in $frame.instancestack.iter_mut().rev()
                {
                    if let Some(inst) = global.instances.get_mut(id)
                    {
                        if let Some(var) = inst.variables.get_mut(&$dirvar.name)
                        {
                            assign_or_return_indexed!($value, var, $indexes, false);
                        }
                        // no need to check for instance function names because they can't be indexed
                    }
                }
            }
        }
        
        macro_rules! assign_or_return {
            ( $value:expr, $var:expr ) =>
            {
                if let Some(value) = $value
                {
                    *$var = value.clone();
                    
                    return None;
                }
                else
                {
                    return Some($var.clone());
                }
            }
        }
        macro_rules! check_frame_dirvar {
            ( $frame:expr, $dirvar:expr, $value:expr ) =>
            {
                // FIXME: do I even want to search up instance stacks rather than just accessing the main one?
                for scope in $frame.scopes.iter_mut().rev()
                {
                    if let Some(var) = scope.get_mut(&$dirvar.name)
                    {
                        assign_or_return!($value, var);
                    }
                }
                for id in $frame.instancestack.iter_mut().rev()
                {
                    if let Some(inst) = global.instances.get_mut(id)
                    {
                        if let Some(var) = inst.variables.get_mut(&$dirvar.name)
                        {
                            assign_or_return!($value, var);
                        }
                        else if let Some(objspec) = global.objects.get(&inst.objtype)
                        {
                            if let Some(funcdat) = objspec.functions.get(&$dirvar.name)
                            {
                                if let Some(_value) = $value
                                {
                                    panic!("error: tried to assign to function `{}` in instance of object type `{}`", $dirvar.name, objspec.name);
                                }
                                else
                                {
                                    let mut mydata = funcdat.clone();
                                    mydata.forcecontext = inst.ident;
                                    return Some(Value::Func(FuncVal{internal : false, internalname : None, predefined : None, userdefdata : Some(mydata)}));
                                }
                            }
                        }
                    }
                }
            }
        }
        match &variable
        {
            Variable::Array(ref arrayvar) =>
            {
                match &arrayvar.location
                {
                    NonArrayVariable::Indirect(ref indirvar) =>
                    {
                        // TODO: deduplicate with macros? (vs. non-array code below)
                        if let Some(instance) = global.instances.get_mut(&indirvar.ident)
                        {
                            if let Some(mut var) = instance.variables.get_mut(&indirvar.name)
                            {
                                assign_or_return_indexed!(value, var, arrayvar.indexes, false);
                            }
                            else
                            {
                                panic!("error: tried to read non-extant variable `{}` in instance `{}`", indirvar.name, indirvar.ident);
                            }
                        }
                        else
                        {
                            panic!("error: tried to access variable `{}` from non-extant instance `{}`", indirvar.name, indirvar.ident);
                        }
                    }
                    NonArrayVariable::Direct(ref dirvar) =>
                    {
                        check_frame_dirvar_arrayed!(self.top_frame, dirvar, value, arrayvar.indexes);
                        for frame in self.frames.iter_mut().rev()
                        {
                            check_frame_dirvar_arrayed!(frame, dirvar, value, arrayvar.indexes);
                        }
                        if let Some(_var) = global.objectnames.get(&dirvar.name)
                        {
                            panic!("error: tried to index into object name as though it was an array");
                        }
                        if let Some(_internal_func) = self.get_internal_function(&dirvar.name)
                        {
                            panic!("error: tried to index into internal function name as though it was an array");
                        }
                        panic!("error: unknown variable `{}`", dirvar.name);
                    }
                    NonArrayVariable::ActualArray(ref array) =>
                    {
                        assign_or_return_indexed!(value, &mut Value::Array(array.clone()), arrayvar.indexes, true);
                    }
                }
            }
            Variable::Indirect(ref indirvar) =>
            {
                // TODO: deduplicate with macros? (vs. array code above)
                if let Some(instance) = global.instances.get_mut(&indirvar.ident)
                {
                    if let Some(var) = instance.variables.get_mut(&indirvar.name)
                    {
                        assign_or_return!(value, var);
                    }
                    else if let Some(objspec) = global.objects.get(&instance.objtype)
                    {
                        if let Some(funcdat) = objspec.functions.get(&indirvar.name)
                        {
                            if let Some(_value) = value
                            {
                                panic!("error: tried to assign to function `{}` in instance of object type `{}`", indirvar.name, objspec.name);
                            }
                            else
                            {
                                let mut mydata = funcdat.clone();
                                mydata.forcecontext = indirvar.ident;
                                return Some(Value::Func(FuncVal{internal : false, internalname : None, predefined : None, userdefdata : Some(mydata)}));
                            }
                        }
                        else
                        {
                            panic!("error: tried to read non-extant variable `{}` in instance `{}`", indirvar.name, indirvar.ident);
                        }
                    }
                    else
                    {
                        panic!("error: tried to read non-extant variable `{}` in instance `{}`", indirvar.name, indirvar.ident);
                    }
                }
                else
                {
                    panic!("error: tried to access variable `{}` from non-extant instance `{}`", indirvar.name, indirvar.ident);
                }
            }
            Variable::Direct(ref dirvar) =>
            {
                check_frame_dirvar!(self.top_frame, dirvar, value);
                for frame in self.frames.iter_mut().rev()
                {
                    check_frame_dirvar!(frame, dirvar, value);
                }
                if let Some(var) = global.objectnames.get(&dirvar.name)
                {
                    if let Some(_value) = value
                    {
                        panic!("error: tried to assign to read-only object name `{}`", dirvar.name);
                    }
                    else
                    {
                        return Some(Value::Number(*var as f64));
                    }
                }
                // TODO: Store actual function pointer instead?
                if let Some(_internal_func) = self.get_internal_function(&dirvar.name)
                {
                    return Some(Value::Func(FuncVal { internal : true, internalname : Some(dirvar.name.clone()), predefined : None, userdefdata : None }));
                }
                
                panic!("error: unknown identifier `{}`", dirvar.name);
            }
        }
    }
}

fn main() -> std::io::Result<()>
{
    let mut file = File::open("grammarsimple.txt")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    
    let mut parser = Parser::new();
    parser.init(&contents);
    
    let mut file2 = File::open("program.txt")?;
    let mut contents2 = String::new();
    file2.read_to_string(&mut contents2)?;
    
    let tokens = parser.tokenize(contents2, false);
    
    if let Some(ref ast) = parser.parse_program(&tokens, false)
    {
        let code = compile_bytecode(ast);
        
        /*
        print_ast(ast);
        for byte in &code
        {
            print!("{:02X} ", byte);
        }
        println!();
        */
        
        let disassembly = disassemble_bytecode(&code, 0, 0);
        for line in disassembly
        {
            println!("{}", line);
        }
        
        let mut global = GlobalState::new(parser.clone());
        
        let mut interpreter = Interpreter::new(code);
        
        while interpreter.step(&mut global){}
    }
    
    Ok(())
}