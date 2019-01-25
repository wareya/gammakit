#![allow(clippy::len_zero)]

use std::collections::{HashMap, HashSet, BTreeSet, VecDeque};
use std::time::Instant;

use crate::{ast::*, grammar::*, strings::*};
use crate::regexholder::RegexHolder;

// For performance reasons (i.e. temporary parse error storage is VERY slow otherwise),
//  we store possible tokens at the point of possible parse errors with a BTreeMap
//  with short strings stored literally as bytes instead of in a String

#[derive(Clone, Debug)]
pub (crate) struct ParseError {
    token : usize, // location of the token that caused the error
    expected : BTreeSet<MiniStr>,
}

impl ParseError {
    pub (crate) fn new(token : usize, text : &str) -> ParseError
    {
        let mut expected = BTreeSet::new();
        expected.insert(MiniStr::from(text));
        ParseError{token, expected}
    }
}

pub (crate) fn build_new_error(myself : &mut Option<ParseError>, token : usize, text : &str)
{
    match myself
    {
        Some(myself) =>
        {
            if token > myself.token
            {
                *myself = ParseError::new(token, text);
            }
            else if token == myself.token
            {
                myself.expected.insert(MiniStr::from(text));
            }
        }
        None => *myself = Some(ParseError::new(token, text))
    }
}

pub (crate) fn build_best_error(myself : &mut Option<ParseError>, other : Option<ParseError>)
{
    match (myself.as_mut(), other)
    {
        (Some(myself), Some(other)) =>
        {
            if other.token > myself.token
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
        (None, Some(other)) => *myself = Some(other),
        _ => {}
    }
}

#[derive(Clone)]
pub (crate) struct GrammarPoint {
    pub (crate) name: String,
    pub (crate) forms: Vec<GrammarForm>,
    pub (crate) istoken: bool,
    pub (crate) precedence : Option<u64> // precedence of left-associative binary operator rules
}

#[derive(Clone)]
/// Provides facilities for turning program text into an AST.
pub struct Parser {
    pub (crate) regex_list : Vec<String>,
    pub (crate) symbol_list : Vec<String>,
    pub (crate) text_list : Vec<String>,
    // token matchers are inserted into both sets and vectors, sets to quickly check for duplicate insertion and vectors are for order
    pub (crate) regex_set : HashSet<String>,
    pub (crate) symbol_set : HashSet<String>,
    pub (crate) text_set : HashSet<String>,
    
    pub (crate) nodetypemap: HashMap<String, GrammarPoint>,
    pub (crate) internal_regexes: RegexHolder,
    inited: bool,
}

fn minierr(mystr : &str) -> String
{
    mystr.to_string()
}

fn plainerr<T>(mystr : &str) -> Result<T, String>
{
    Err(minierr(mystr))
}

type ParseInfo = (Option<ASTNode>, usize, Option<ParseError>);
type ParseVecInfo = (Option<Vec<ASTNode>>, usize, Option<ParseError>);

impl Default for Parser {
    fn default() -> Parser
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
            inited: false
        }
    }
}
impl Parser {
    pub fn new_from_default() -> Result<Parser, String>
    {
        let mut parser = Parser::default();
        parser.init(super::grammar::default_grammar())?;
        Ok(parser)
    }
    pub fn new_from_grammar(grammar : &str) -> Result<Parser, String>
    {
        let mut parser = Parser::default();
        parser.init(grammar)?;
        Ok(parser)
    }
    fn init(&mut self, text: &str) -> Result<(), String>
    {
        let start_time = Instant::now();
        
        let mut lines : VecDeque<String> = text.lines().map(|x| x.to_string()).collect();
        // guarantee the last line is ""
        lines.push_back("".to_string());
    
        while lines.len() > 0
        {
            macro_rules! pop { () => { lines.pop_front().ok_or_else(|| "tried to access past end of program text".to_string()) }; }
            
            let mut line : String = pop!()?;
            if line == ""
            {
                continue;
            }
            let captures = self.internal_regexes.captures("([a-zA-Z_][a-zA-Z_0-9]*):[ ]*(TOKEN)?[ ]*(LEFTBINEXPR [0-9]+)?", &line)
                .ok_or_else(|| minierr(&format!("general syntax error\noffending line:\n{}", line)))?;
            let name = captures.get(1).ok_or_else(|| minierr("unreachable error in parser init getting rule name"))?.as_str().to_string();
            let istoken = captures.get(2).is_some();
            let precedence =
            match captures.get(3)
            {
                Some(x) => Some(slice_to_end(x.as_str(), 12).parse::<u64>().or_else(|_| plainerr("error: LEFTBINEXPR argument must be a positive integer"))?),
                None => None
            };
            // last line is guaranteed to be "" which means we are unable to pop past the end here
            let mut nodetype : GrammarPoint = GrammarPoint{name, forms: Vec::new(), istoken, precedence};
            line = pop!()?;
            while line != ""
            {
                nodetype.forms.push(GrammarForm::new(&line, self, istoken)?);
                line = pop!()?;
            }
            if self.nodetypemap.contains_key(&nodetype.name)
            {
                return plainerr(&format!("error: node type `{}` declared twice", nodetype.name));
            }
            self.nodetypemap.insert(nodetype.name.clone(), nodetype);
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
                        GrammarToken::SeparatorNameList{text, ..} => text.clone(),
                        _ => "".to_string()
                    };
                    if name != "" && !self.nodetypemap.contains_key(&name)
                    {
                        return plainerr(&format!("error: node name {} is used without actually defined", name));
                    }
                }
            }
        }
        if !self.nodetypemap.contains_key("program")
        {
            return plainerr("error: grammar does not define name \"program\"");
        }
        
        self.symbol_list.sort_by_key(|text| -(text.len() as i64));
        self.text_list  .sort_by_key(|text| -(text.len() as i64));
        
        self.inited = true;
        
        println!("init took {:?}", Instant::now().duration_since(start_time));
        
        Ok(())
    }
    
    // FIXME: change it to not be line-based; seek to the next newline instead. necessary for things like strings containing newline literals, which should definitely be supported.
    pub (crate) fn tokenize(&mut self, lines : &[String], silent: bool) -> Result<VecDeque<LexToken>, String>
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
                if let Some(signal) = line.get(offset..offset+2)
                {
                    if signal == "*/" && in_multiline_comment
                    {
                        in_multiline_comment = false;
                        offset += 2;
                        continue;
                    }
                    else if signal == "/*"
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
                    if let Some(segment) = line.get(offset..offset+text.len())
                    {
                        if segment == text.as_str()
                        {
                            ret.push_back(LexToken{text : text.clone(), line : linecount, position : offset});
                            offset += text.len();
                            continue_the_while = true;
                            break;
                        }
                    }
                }
                if continue_the_while { continue; }
                for text in &self.text_list
                {
                    if let Some(segment) = line.get(offset..offset+text.len())
                    {
                        if segment == text.as_str()
                        {
                            // don't tokenize the beginnings of names as actual names
                            if offset + text.len() + 1 > line.len() && self.internal_regexes.is_exact(r"[a-zA-Z0-9_]", &slice(&line, (offset+text.len()) as i64, (offset+text.len()+1) as i64))
                            {
                                continue;
                            }
                            ret.push_back(LexToken{text : text.clone(), line : linecount, position : offset});
                            offset += text.len();
                            continue_the_while = true;
                            break;
                        }
                    }
                }
                if continue_the_while { continue; }
                return plainerr(&format!("failed to tokenize program\noffending line:\n{}", line));
            }
            linecount += 1;
        }
        
        if !silent
        {
            println!("lex took {:?}", Instant::now().duration_since(start_time));
        }
        
        Ok(ret)
    }

    // attempts to parse a token list as a particular form of a grammar point
    fn parse_form(&self, tokens : &VecDeque<LexToken>, index : usize, form : &GrammarForm, formname : Option<&str>) -> Result<ParseVecInfo, String>
    {
        if tokens.len() == 0
        {
            return Ok((None, 0, None));
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
                    let kind = self.nodetypemap.get(text).ok_or_else(|| minierr(&format!("internal error: failed to find node type {} used by some grammar form", text)))?;
                    
                    let (bit, consumed, error) = self.parse(&tokens, index+totalconsumed, kind)?;
                    build_best_error(&mut latesterror, error);
                    if let Some(node) = bit
                    {
                        nodes.push(node);
                        totalconsumed += consumed;
                    }
                    else
                    {
                        return Ok((defaultreturn.0, defaultreturn.1, latesterror));
                    }
                }
                GrammarToken::NameList(text) =>
                {
                    let kind = self.nodetypemap.get(text).ok_or_else(|| minierr(&format!("internal error: failed to find node type {} used by some grammar form", text)))?;
                    
                    let (mut bit, mut consumed, mut error) = self.parse(&tokens, index+totalconsumed, kind)?;
                    build_best_error(&mut latesterror, error);
                    if bit.is_none()
                    {
                        return Ok((defaultreturn.0, defaultreturn.1, latesterror));
                    }
                    while let Some(node) = bit
                    {
                        nodes.push(node);
                        totalconsumed += consumed;
                        
                        let tuple = self.parse(&tokens, index+totalconsumed, kind)?;
                        bit = tuple.0;
                        consumed = tuple.1;
                        error = tuple.2;
                        
                        build_best_error(&mut latesterror, error);
                    }
                }
                GrammarToken::OptionalName(text) =>
                {
                    let kind = self.nodetypemap.get(text).ok_or_else(|| minierr(&format!("internal error: failed to find node type {} used by some grammar form", text)))?;
                    
                    let (bit, consumed, error) = self.parse(&tokens, index+totalconsumed, kind)?;
                    build_best_error(&mut latesterror, error);
                    if let Some(node) = bit
                    {
                        nodes.push(node);
                        totalconsumed += consumed;
                    }
                }
                GrammarToken::OptionalNameList(text) =>
                {
                    let kind = self.nodetypemap.get(text).ok_or_else(|| minierr(&format!("internal error: failed to find node type {} used by some grammar form", text)))?;
                    
                    let (mut bit, mut consumed, mut error) = self.parse(&tokens, index+totalconsumed, kind)?;
                    build_best_error(&mut latesterror, error);
                    while let Some(node) = bit
                    {
                        nodes.push(node);
                        totalconsumed += consumed;
                        
                        let tuple = self.parse(&tokens, index+totalconsumed, kind)?;
                        bit = tuple.0;
                        consumed = tuple.1;
                        error = tuple.2;
                        
                        build_best_error(&mut latesterror, error);
                    }
                }
                GrammarToken::SeparatorNameList{text, separator} =>
                {
                    let kind = self.nodetypemap.get(text).ok_or_else(|| minierr(&format!("internal error: failed to find node type {} used by some grammar form", text)))?;
                    
                    let (mut bit, mut consumed, mut error) = self.parse(&tokens, index+totalconsumed, kind)?;
                    build_best_error(&mut latesterror, error);
                    if bit.is_none()
                    {
                        return Ok((defaultreturn.0, defaultreturn.1, latesterror));
                    }
                    while let Some(node) = bit
                    {
                        nodes.push(node);
                        totalconsumed += consumed;
                        
                        if let Some(check_separator) = tokens.get(index+totalconsumed)
                        {
                            if check_separator.text == *separator
                            {
                                totalconsumed += 1;
                                
                                let tuple = self.parse(&tokens, index+totalconsumed, kind)?;
                                bit = tuple.0;
                                consumed = tuple.1;
                                error = tuple.2;
                                
                                build_best_error(&mut latesterror, error);
                                
                                // undo separator drain if right-hand rule parse failed
                                if bit.is_none()
                                {
                                    totalconsumed -= 1;
                                }
                                continue;
                            }
                        }
                        break;
                    }
                }
                GrammarToken::Plain(text) =>
                {
                    if let Some(token) = tokens.get(index+totalconsumed)
                    {
                        if token.text == *text
                        {
                            nodes.push(ASTNode{text : token.text.to_string(), line : token.line, position : token.position, isparent: false, children : Vec::new(), precedence : None});
                            totalconsumed += 1;
                            continue;
                        }
                    }
                    build_new_error(&mut latesterror, index+totalconsumed, &text);
                    return Ok((defaultreturn.0, defaultreturn.1, latesterror));
                }
                GrammarToken::Regex(text) =>
                {
                    if let Some(token) = tokens.get(index+totalconsumed)
                    {
                        if self.internal_regexes.is_exact_immut(text, &token.text)?
                        {
                            nodes.push(ASTNode{text : token.text.to_string(), line : token.line, position : token.position, isparent: false, children : Vec::new(), precedence : None});
                            totalconsumed += 1;
                            continue;
                        }
                    }
                    build_new_error(&mut latesterror, index+totalconsumed, formname.unwrap_or(&text));
                    return Ok((defaultreturn.0, defaultreturn.1, latesterror));
                }
                GrammarToken::RestIsOptional =>
                {
                    defaultreturn = (Some(nodes.clone()), totalconsumed);
                }
            }
        }
        
        Ok((Some(nodes), totalconsumed, latesterror))
    }

    // attempts to parse a token list as each form of a grammar point in order and uses the first valid one
    fn parse(&self, tokens : &VecDeque<LexToken>, index : usize, nodetype : &GrammarPoint) -> Result<ParseInfo, String>
    {
        if tokens.len() == 0
        {
            return Ok((None, 0, None));
        }
        
        let mut latesterror : Option<ParseError> = None;
        
        for form in &nodetype.forms
        {
            let sentname =
            if nodetype.istoken
            {
                Some(nodetype.name.as_str())
            }
            else
            {
                None
            };
            let (nodes, consumed, error) = self.parse_form(&tokens, index, form, sentname)?;
            build_best_error(&mut latesterror, error);
            if let Some(token) = tokens.get(index)
            {
                if let Some(nodes) = nodes
                {
                    return Ok((Some(ASTNode{text : nodetype.name.clone(), line : token.line, position : token.position, isparent : true, children : nodes, precedence : nodetype.precedence}), consumed, latesterror));
                }
            }
        }
        Ok((None, 0, latesterror))
    }
    fn rotate(ast : &mut ASTNode) -> Result<(), String>
    {
        if !(ast.isparent && ast.children.len() == 3 && ast.child(2)?.isparent && ast.child(2)?.children.len() >= 1)
        {
            return plainerr("internal error: attempted to rotate AST node for which the conditions of AST rotation were not satisfied");
        }
        let mut node_holder = dummy_astnode();
        // tree rotation around self, child 0, and child 2
        std::mem::swap(&mut node_holder, ast.child_mut(2)?); // detach right from under left (leaving dummy on left)
        std::mem::swap(ast.child_mut(2)?, node_holder.child_mut(0)?); // move betweener from right to left (leaving dummy on right)
        std::mem::swap(ast, node_holder.child_mut(0)?); // attach left to under right (leaving dummy on root)
        std::mem::swap(ast, &mut node_holder); // attach right to root (leaving dummy on node_holder)
        Ok(())
    }
    fn parse_rotate_associativity_binexpr(&self, ast : &mut ASTNode) -> Result<bool, String>
    {
        fn is_rotatable_binexpr(a : &ASTNode) -> bool
        {
            a.isparent && a.children.len() == 3 && a.precedence.is_some()
        }
        fn compatible_associativity(a : &ASTNode, b : &ASTNode) -> Result<bool, String>
        {
            Ok(a.isparent && b.isparent && a.child(0)?.precedence == b.child(0)?.precedence)
        }
        if is_rotatable_binexpr(ast) && is_rotatable_binexpr(ast.child(2)?) && compatible_associativity(ast, ast.child(2)?)?
        {
            Parser::rotate(ast)?;
            Ok(true)
        }
        else
        {
            Ok(false)
        }
    }
    fn parse_fix_associativity(&self, ast : &mut ASTNode) -> Result<(), String>
    {
        if ast.isparent
        {
            if self.parse_rotate_associativity_binexpr(ast)?
            {
                self.parse_fix_associativity(ast)?;
            }
            else
            {
                for mut child in &mut ast.children
                {
                    self.parse_fix_associativity(&mut child)?;
                }
            }
        }
        Ok(())
    }
    fn parse_tweak_ast(&self, ast : &mut ASTNode) -> Result<(), String>
    {
        if ast.isparent
        {
            if ast.text == "statement" && !ast.last_child()?.isparent && ast.last_child()?.text == ";"
            {
                ast.children.pop();
            }
            while (ast.text.starts_with("binexpr_") || ast.text == "simplexpr" || ast.text == "parenexpr" || ast.text == "supersimplexpr") && ast.children.len() == 1
            {
                let mut temp = Vec::new();
                std::mem::swap(&mut temp, &mut ast.children);
                let dummy = temp.get_mut(0).ok_or_else(|| minierr("internal error: could not access child that was supposed to be there in expression summarization"))?;
                std::mem::swap(ast, dummy);
            }
            
            match ast.text.as_str()
            {
                "objdef" =>
                {
                    if ast.children.len() < 3
                    {
                        return plainerr("internal error: objdef AST node does not have at least 3 children");
                    }
                    for child in ast.child_slice(3, -1)?
                    {
                        if matches!(child.child(1)?.child(0)?.text.as_str(), "create" | "destroy")
                           && (child.child(3)?.isparent || child.child(3)?.text != ")")
                        {
                            return plainerr(&format!("error: `{}` function of object must not have any arguments", child.child(1)?.child(0)?.text));
                        }
                    }
                }
                "funccall" =>
                {
                    if ast.child(0)?.last_child()?.child(0)?.text != "funcargs"
                    {
                        return plainerr("error: tried to use non-function expression as a funccall statement");
                    }
                }
                _ => {}
            }
            
            for mut child in &mut ast.children
            {
                self.parse_tweak_ast(&mut child)?;
            }
        }
        Ok(())
    }
    
    pub fn parse_program(&self, tokens : &VecDeque<LexToken>, lines : &[String], silent: bool) -> Result<Option<ASTNode>, String>
    {
        let start_time = Instant::now();
        
        if !silent
        {
            println!("parsing...");
        }
        if let Some(program_type) = self.nodetypemap.get("program")
        {
            let (raw_ast, consumed, latesterror) = self.parse(&tokens, 0, program_type)?;
            if !silent
            {
                println!("successfully parsed {} out of {} tokens", consumed, tokens.len());
                println!("parse took {:?}", Instant::now().duration_since(start_time));
            }
            
            if consumed != tokens.len() || raw_ast.is_none()
            {
                if let Some(mut error) = latesterror
                {
                    let mut expected : Vec<String> = error.expected.into_iter().map(|x| x.to_string()).collect();
                    expected.sort();
                    let onepast = error.token == tokens.len();
                    if onepast
                    {
                        error.token -= 1;
                    }
                    if expected.len() == 1
                    {
                        if let Some(expect) = expected.get(0)
                        {
                            println!("error: expected `{}`", expect);
                        }
                        else
                        {
                            println!("internal error: failed to grab expected symbol that was supposed to be there while printing parser error");
                        }
                    }
                    else
                    {
                        println!("error: expected one of `{}`", expected.join("`, `"));
                    }
                    if let Some(token) = tokens.get(error.token)
                    {
                        let linenum = token.line;
                        let mut position = token.position;
                        if onepast
                        {
                            position += 1;
                        }
                        if let Some(line) = lines.get(linenum-1)
                        {
                            println!("context on line {}:\n{}\n{}^", linenum, line, " ".repeat(position));
                        }
                        else
                        {
                            println!("internal error: failed to grab context text for parse error");
                        }
                        if onepast
                        {
                            println!("note: this is past the end of your program; you probably have an unclosed block delimiter (or something similar) way, way up there somewhere");
                        }
                    }
                    else
                    {
                        println!("internal error: failed to grab context info for parse error; token number {} out of {}", error.token, tokens.len());
                    }
                }
                else
                {
                    println!("error: unexpected or malformed expression");
                    if let Some(token) = tokens.get(consumed)
                    {
                        println!("(line {})\n(position {})", token.line, token.position);
                    }
                    else
                    {
                        println!("internal error: failed to grab context for parse error");
                    }
                }
                
                Ok(None)
            }
            else if let Some(mut ast) = raw_ast
            {
                if !silent
                {
                    println!("fixing associativity...");
                }
                self.parse_fix_associativity(&mut ast)?;
                
                if !silent
                {
                    println!("tweaking AST...");
                }
                self.parse_tweak_ast(&mut ast)?;
                
                if !silent
                {
                    println!("all good!");
                }
                
                Ok(Some(ast))
            }
            else
            {
                plainerr("internal error: parser did not return AST despite it failing is_none() check")
            }
        }
        else
        {
            plainerr("error: grammar does not define \"program\" node type")
        }
    }
}
