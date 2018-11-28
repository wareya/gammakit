use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::time::Instant;

use std::hint::unreachable_unchecked;

use super::ast::*;
use super::grammar::*;
use super::regexholder::RegexHolder;
use super::strings::*;
    
#[derive(Clone)]
#[derive(Debug)]
pub struct ParseError {
    token : usize, // location of the token that caused the error
    expected : HashSet<String>,
}

impl ParseError {
    pub fn new(token : usize, text : &str) -> ParseError
    {
        return ParseError{token, expected : vec!(text.to_string()).into_iter().collect()}
    }
}

pub fn build_best_error(myself : &mut Option<ParseError>, other : Option<ParseError>)
{
    if let Some(other) = other
    {
        if myself.is_some()
        {
            if let Some(myself) = myself.as_mut()
            {
                if other.token < myself.token
                {
                    return;
                }
                else if other.token > myself.token
                {
                    *myself = other;
                }
                else if other.token == myself.token
                {
                    for text in other.expected
                    {
                        myself.expected.insert(text);
                    }
                }
            }
        }
        else
        {
            *myself = Some(other);
        }
    }
}
#[derive(Clone)]
pub struct Parser {
    pub regex_list : Vec<String>,
    pub symbol_list : Vec<String>,
    pub text_list : Vec<String>,
    // token matchers are inserted into both sets and vectors, sets to quickly check for duplicate insertion and vectors are for order
    pub regex_set : HashSet<String>,
    pub symbol_set : HashSet<String>,
    pub text_set : HashSet<String>,
    
    pub nodetypemap: HashMap<String, GrammarPoint>,
    pub internal_regexes: RegexHolder,
    inited: bool,
}

impl Parser {
    pub fn new() -> Parser
    {
        Parser {
            regex_list: Vec::new(),
            symbol_list: Vec::new(),
            text_list: Vec::new(),
            regex_set: HashSet::new(),
            symbol_set: HashSet::new(),
            text_set: HashSet::new(),
            nodetypemap: HashMap::new(),
            internal_regexes: RegexHolder::new(),
            inited: false }
    }
    
    pub fn init(&mut self, text: &str)
    {
        let start_time = Instant::now();
        
        let mut lines : VecDeque<String> = text.lines().map(|x| x.to_string()).collect();
        // guarantee the last line is ""
        lines.push_back("".to_string());
    
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
                let mut nodetype : GrammarPoint = GrammarPoint{name, forms: Vec::new(), istoken};
                line = pop!();
                while line != ""
                {
                    nodetype.forms.push(GrammarForm::new(&line, self, istoken));
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
        
        for regex in &self.regex_set
        {
            self.internal_regexes.prepare_exact(&regex);
        }
        
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
                        GrammarToken::OptionalNameList(text) |
                        GrammarToken::SeparatorNameList{text, ..} => {text.clone()}
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
            let point = &self.nodetypemap[name];
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
                            GrammarToken::Op{text, ..} =>
                            { print!("op:{} ", text); }
                            GrammarToken::RestIsOptional => { print!(">>? "); }
                        }
                    }
                    println!();
                }
                println!();
            }
        }
        if !self.nodetypemap.contains_key("program")
        {
            panic!("error: grammar does not define name \"program\"");
        }
        
        self.symbol_list.sort_by_key(|text| -(text.len() as i64));
        self.text_list  .sort_by_key(|text| -(text.len() as i64));
        
        self.inited = true;
        
        println!("init took {:?}", Instant::now().duration_since(start_time));
    }
    
    // FIXME: change it to not be line-based; seek to the next newline instead. necessary for things like strings containing newline literals, which should definitely be supported.
    pub fn tokenize(&mut self, lines : &[String], silent: bool) -> VecDeque<LexToken>
    {
        let start_time = Instant::now();
        
        let mut ret : VecDeque<LexToken> = VecDeque::new();
        let mut linecount = 1;
        
        let mut in_multiline_comment = false;
        
        for line in lines
        {
            let mut offset : usize = 0; // in bytes
            while offset < line.len() // also in bytes
            {
                // check for comments before doing anything else
                if offset+1 < line.len()
                {
                    let signal = &line[offset..offset+2];
                    if in_multiline_comment
                    {
                        if signal == "*/"
                        {
                            in_multiline_comment = false;
                            offset += 2;
                            continue;
                        }
                    }
                    else // not currently in a multiline comment
                    {
                        if signal == "/*"
                        {
                            in_multiline_comment = true;
                            offset += 2;
                            continue;
                        }
                        else if signal == "//"
                        {
                            break;
                        }
                    }
                }
                if in_multiline_comment
                {
                    offset += 1;
                    continue;
                }
                // check for whitespace before doing any tokens
                if let Some(text) = self.internal_regexes.match_at("[ \r\n\t]+", &line, offset)
                {
                    offset += text.len();
                    continue;
                }
                
                let mut continue_the_while = false;
                for rule in &self.regex_list
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
                for text in &self.symbol_list
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
                for text in &self.text_list
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
    fn parse_form(&self, tokens : &VecDeque<LexToken>, index : usize, form : &GrammarForm) -> (Option<Vec<ASTNode>>, usize, Option<ParseError>)
    {
        if tokens.len() == 0
        {
            return (None, 0, None);
        }
        
        let mut nodes : Vec<ASTNode> = Vec::new();
        let mut totalconsumed : usize = 0;
        
        let mut latesterror : Option<ParseError> = None;
        
        let mut defaultreturn : (Option<Vec<ASTNode>>, usize) = (None, 0);
        
        for part in &form.tokens
        {
            match part
            {
                GrammarToken::Name(text) =>
                {
                    if !self.nodetypemap.contains_key(text)
                    {
                        panic!("internal error: failed to find node type {} used by some grammar form", text);
                    }
                    let (bit, consumed, error) = self.parse(&tokens, index+totalconsumed, &self.nodetypemap[text]);
                    build_best_error(&mut latesterror, error);
                    if bit.is_some()
                    {
                        let node = bit.unwrap();
                        nodes.push(node);
                        totalconsumed += consumed;
                    }
                    else
                    {
                        return (defaultreturn.0, defaultreturn.1, latesterror);
                    }
                }
                GrammarToken::NameList(text) =>
                {
                    if !self.nodetypemap.contains_key(text)
                    {
                        panic!("internal error: failed to find node type {} used by some grammar form", text);
                    }
                    let (mut bit, mut consumed, mut error) = self.parse(&tokens, index+totalconsumed, &self.nodetypemap[text]);
                    build_best_error(&mut latesterror, error);
                    if bit.is_none()
                    {
                        return (defaultreturn.0, defaultreturn.1, latesterror);
                    }
                    while bit.is_some()
                    {
                        let node = bit.unwrap();
                        nodes.push(node);
                        totalconsumed += consumed;
                        
                        let tuple = self.parse(&tokens, index+totalconsumed, &self.nodetypemap[text]);
                        bit = tuple.0;
                        consumed = tuple.1;
                        error = tuple.2;
                        
                        build_best_error(&mut latesterror, error);
                    }
                }
                GrammarToken::OptionalName(text) =>
                {
                    if !self.nodetypemap.contains_key(text)
                    {
                        panic!("internal error: failed to find node type {} used by some grammar form", text);
                    }
                    let (bit, consumed, mut error) = self.parse(&tokens, index+totalconsumed, &self.nodetypemap[text]);
                    build_best_error(&mut latesterror, error);
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
                    let (mut bit, mut consumed, mut error) = self.parse(&tokens, index+totalconsumed, &self.nodetypemap[text]);
                    build_best_error(&mut latesterror, error);
                    while bit.is_some()
                    {
                        let node = bit.unwrap();
                        nodes.push(node);
                        totalconsumed += consumed;
                        
                        let tuple = self.parse(&tokens, index+totalconsumed, &self.nodetypemap[text]);
                        bit = tuple.0;
                        consumed = tuple.1;
                        error = tuple.2;
                        
                        build_best_error(&mut latesterror, error);
                    }
                }
                GrammarToken::SeparatorNameList{text, separator} =>
                {
                    if !self.nodetypemap.contains_key(text)
                    {
                        panic!("internal error: failed to find node type {} used by some grammar form", text);
                    }
                    let (mut bit, mut consumed, mut error) = self.parse(&tokens, index+totalconsumed, &self.nodetypemap[text]);
                    build_best_error(&mut latesterror, error);
                    if bit.is_none()
                    {
                        return (defaultreturn.0, defaultreturn.1, latesterror);
                    }
                    while bit.is_some()
                    {
                        let node = bit.unwrap();
                        nodes.push(node);
                        totalconsumed += consumed;
                        
                        if tokens.len() <= index+totalconsumed { break; }
                        if tokens[index+totalconsumed].text != *separator { break; }
                        totalconsumed += 1;
                        
                        let tuple = self.parse(&tokens, index+totalconsumed, &self.nodetypemap[text]);
                        bit = tuple.0;
                        consumed = tuple.1;
                        error = tuple.2;
                        
                        build_best_error(&mut latesterror, error);
                        
                        // undo separator drain if right-hand rule parse failed
                        if bit.is_none()
                        {
                            totalconsumed -= 1;
                        }
                    }
                }
                GrammarToken::Plain(text) =>
                {
                    if tokens.len() <= index+totalconsumed
                    {
                        let error = Some(ParseError::new(index+totalconsumed, &text));
                        build_best_error(&mut latesterror, error);
                        return (defaultreturn.0, defaultreturn.1, latesterror);
                    }
                    let token_text = &tokens[index+totalconsumed].text;
                    //println!("comparing {} to {}", token_text, *text);
                    if token_text == text
                    {
                        nodes.push(ASTNode{text : token_text.to_string(), line : tokens[index+totalconsumed].line, position : tokens[index+totalconsumed].position, isparent: false, children : Vec::new(), opdata : dummy_opdata()});
                        totalconsumed += 1;
                    }
                    else
                    {
                        let error = Some(ParseError::new(index+totalconsumed, &text));
                        build_best_error(&mut latesterror, error);
                        return (defaultreturn.0, defaultreturn.1, latesterror);
                    }
                }
                GrammarToken::Regex(text) =>
                {
                    if tokens.len() <= index+totalconsumed
                    {
                        let error = Some(ParseError::new(index+totalconsumed, &text));
                        build_best_error(&mut latesterror, error);
                        return (defaultreturn.0, defaultreturn.1, latesterror);
                    }
                    let token_text = &tokens[index+totalconsumed].text;
                    //println!("regex comparing {} to {}", token_text, *text);
                    if self.internal_regexes.is_exact_immut(text, token_text)
                    {
                        nodes.push(ASTNode{text : token_text.to_string(), line : tokens[index+totalconsumed].line, position : tokens[index+totalconsumed].position, isparent: false, children : Vec::new(), opdata : dummy_opdata()});
                        totalconsumed += 1;
                    }
                    else
                    {
                        let error = Some(ParseError::new(index+totalconsumed, &text));
                        build_best_error(&mut latesterror, error);
                        return (defaultreturn.0, defaultreturn.1, latesterror);
                    }
                }
                GrammarToken::Op{text, assoc, precedence} =>
                {
                    if tokens.len() <= index+totalconsumed
                    {
                        let error = Some(ParseError::new(index+totalconsumed, &text));
                        build_best_error(&mut latesterror, error);
                        return (defaultreturn.0, defaultreturn.1, latesterror);
                    }
                    let token_text = &tokens[index+totalconsumed].text;
                    if token_text == text
                    {
                        nodes.push(ASTNode{text : token_text.to_string(), line : tokens[index+totalconsumed].line, position : tokens[index+totalconsumed].position, isparent: false, children : Vec::new(), opdata : OpData{isop : true, assoc: *assoc, precedence: *precedence}});
                        totalconsumed += 1;
                    }
                    else
                    {
                        let error = Some(ParseError::new(index+totalconsumed, &text));
                        build_best_error(&mut latesterror, error);
                        return (defaultreturn.0, defaultreturn.1, latesterror);
                    }
                }
                GrammarToken::RestIsOptional =>
                {
                    defaultreturn = (Some(nodes.clone()), totalconsumed);
                }
            }
        }
        
        return (Some(nodes), totalconsumed, latesterror);
    }

    // attempts to parse a token list as each form of a grammar point in order and uses the first valid one
    fn parse(&self, tokens : &VecDeque<LexToken>, index : usize, nodetype : &GrammarPoint) -> (Option<ASTNode>, usize, Option<ParseError>)
    {
        if tokens.len() == 0
        {
            return (None, 0, None);
        }
        
        let mut latesterror : Option<ParseError> = None;
        
        for form in &nodetype.forms
        {
            let (nodes, consumed, error) = self.parse_form(&tokens, index, form);
            build_best_error(&mut latesterror, error);
            if let Some(nodes) = nodes
            {
                return (Some(ASTNode{text : nodetype.name.clone(), line : tokens[index].line, position : tokens[index].position, isparent : true, children : nodes, opdata : dummy_opdata()}), consumed, latesterror);
            }
        }
        return (None, 0, latesterror);
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
            match ast.text.as_str()
            {
                "funcargs" =>
                {
                    if ast.children.len() >= 2
                    && ast.children.first().unwrap().text == "(" && !ast.children.first().unwrap().isparent
                    && ast.children.last().unwrap().text == ")" && !ast.children.last().unwrap().isparent
                    {
                        ast.children.pop();
                        ast.children.remove(0);
                    }
                }
                "funccall" =>
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
                }
                "arrayref" =>
                {
                    while ast.children.len() > 2
                    {
                        let left = ASTNode{text: ast.text.clone(), line: ast.children[0].line, position: ast.children[0].position, isparent: true, children: ast.children.drain(0..2).collect(), opdata: dummy_opdata()};
                        ast.children.insert(0, left);
                    }
                }
                "rhunexpr" =>
                {
                    while ast.children.len() > 2
                    {
                        let left = ASTNode{text: ast.text.clone(), line: ast.children[0].line, position: ast.children[0].position, isparent: true, children: ast.children.drain(0..2).collect(), opdata: dummy_opdata()};
                        ast.children.insert(0, left);
                    }
                    
                    assert!(ast.children.len() == 2);
                    
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
                "ifcondition" | "whilecondition" | "withstatement" =>
                {
                    assert!(ast.children.len() >= 4);
                    ast.children.remove(3);
                    ast.children.remove(1);
                }
                _ => {}
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
    pub fn parse_program(&self, tokens : &VecDeque<LexToken>, lines : &[String], silent: bool) -> Option<ASTNode>
    {
        let start_time = Instant::now();
        
        if !silent
        {
            println!("parsing...");
        }
        let (raw_ast, consumed, latesterror) = self.parse(&tokens, 0, &self.nodetypemap["program"]);
        if !silent
        {
            println!("successfully parsed {} out of {} tokens", consumed, tokens.len());
            println!("parse took {:?}", Instant::now().duration_since(start_time));
        }
        
        if consumed != tokens.len() || raw_ast.is_none()
        {
            if let Some(error) = latesterror
            {
                let mut expected : Vec<String> = error.expected.iter().cloned().collect();
                expected.sort();
                if error.expected.len() == 1
                {
                    println!("error: expected `{}`", expected[0]);
                }
                else
                {
                    println!("error: expected one of `{}`", expected.join("`, `"));
                }
                let token = tokens.get(error.token).unwrap().clone();
                let linenum = token.line;
                let position = token.position;
                println!("context:\n{}\n{}^", lines[linenum-1], " ".repeat(position));
                //println!("(token {})", error.token);
                //println!("(line {})", tokens.get(error.token).unwrap().line);
                //println!("(position {})", );
            }
            else
            {
                println!("error: unexpected or malformed expression");
                println!("(line {})", tokens.get(consumed).unwrap().line);
                println!("(position {})", tokens.get(consumed).unwrap().position);
            }
            
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
