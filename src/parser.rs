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
        (None, Some(other)) =>
        {
            *myself = Some(other)
        }
        _ => {}
    }
}
#[derive(Clone)]
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
    pub fn new() -> Parser
    {
        Parser::default()
    }
    pub fn init(&mut self, text: &str) -> Result<(), String>
    {
        let start_time = Instant::now();
        
        let mut lines : VecDeque<String> = text.lines().map(|x| x.to_string()).collect();
        // guarantee the last line is ""
        lines.push_back("".to_string());
    
        while lines.len() > 0
        {
            macro_rules! pop {
                () => { lines.pop_front().ok_or_else(|| "tried to access past end of program text".to_string()) };
            }
            
            let mut line : String = pop!()?;
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
                line = pop!()?;
                while line != ""
                {
                    nodetype.forms.push(GrammarForm::new(&line, self, istoken)?);
                    line = pop!()?;
                }
                if !self.nodetypemap.contains_key(&nodetype.name)
                {
                    self.nodetypemap.insert(nodetype.name.clone(), nodetype);
                }
                else
                {
                    return plainerr(&format!("error: node type `{}` declared twice", nodetype.name));
                }
            }
            else
            {
                return plainerr(&format!("general syntax error\noffending line:\n{}", line));
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
                            if offset + text.len() + 1 > line.len()
                               && self.internal_regexes.is_exact(r"[a-zA-Z0-9_]", &slice(&line, (offset+text.len()) as i64, (offset+text.len()+1) as i64))
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
                            nodes.push(ASTNode{text : token.text.to_string(), line : token.line, position : token.position, isparent: false, children : Vec::new(), opdata : dummy_opdata()});
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
                            nodes.push(ASTNode{text : token.text.to_string(), line : token.line, position : token.position, isparent: false, children : Vec::new(), opdata : dummy_opdata()});
                            totalconsumed += 1;
                            continue;
                        }
                    }
                    if let Some(formname) = formname
                    {
                        build_new_error(&mut latesterror, index+totalconsumed, formname);
                    }
                    else
                    {
                        build_new_error(&mut latesterror, index+totalconsumed, &text);
                    }
                    return Ok((defaultreturn.0, defaultreturn.1, latesterror));
                }
                GrammarToken::Op{text, assoc, precedence} =>
                {
                    if let Some(token) = tokens.get(index+totalconsumed)
                    {
                        if token.text == *text
                        {
                            nodes.push(ASTNode{text : token.text.to_string(), line : token.line, position : token.position, isparent: false, children : Vec::new(), opdata : OpData{isop : true, assoc: *assoc, precedence: *precedence}});
                            totalconsumed += 1;
                            continue;
                        }
                    }
                    build_new_error(&mut latesterror, index+totalconsumed, &text);
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
                    return Ok((Some(ASTNode{text : nodetype.name.clone(), line : token.line, position : token.position, isparent : true, children : nodes, opdata : dummy_opdata()}), consumed, latesterror));
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
            a.isparent && a.children.len() == 3 && a.text.starts_with("binexpr_")
        }
        fn compatible_associativity(a : &ASTNode, b : &ASTNode) -> Result<bool, String>
        {
            Ok(
            a.isparent && b.isparent
            && a.child(0)?.opdata.isop
            && b.child(0)?.opdata.isop
            && a.child(0)?.opdata.assoc == 1
            && b.child(0)?.opdata.assoc == 1
            && a.child(0)?.opdata.precedence == b.child(0)?.opdata.precedence
            )
        }
        if is_rotatable_binexpr(ast) && is_rotatable_binexpr(ast.child(2)?) && compatible_associativity(ast.child(1)?, ast.child(2)?.child(1)?)?
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
            while (ast.text.starts_with("binexpr_") || ast.text == "simplexpr" || ast.text == "supersimplexpr") && ast.children.len() == 1
            {
                // FIXME no idea if this works lol
                let mut temp = Vec::new();
                std::mem::swap(&mut temp, &mut ast.children);
                let dummy = temp.get_mut(0).ok_or_else(|| minierr("internal error: could not access child that was supposed to be there in expression summarization"))?;
                std::mem::swap(ast, dummy);
            }
            match ast.text.as_str()
            {
                "funcargs" =>
                {
                    if ast.children.len() >= 2
                    && ast.child(0)?.text == "(" && !ast.child(0)?.isparent
                    && ast.last_child()?.text == ")" && !ast.last_child()?.isparent
                    {
                        ast.children.pop();
                        ast.children.remove(0);
                    }
                }
                "funccall" =>
                {
                    if ast.children.len() == 1
                    {
                        self.parse_tweak_ast(ast.child_mut(0)?)?;
                        if ast.child(0)?.text == "arrayexpr"
                        {
                            return plainerr("error: tried to use array indexing expression as statement");
                        }
                        if ast.child(0)?.text == "indirection"
                        {
                            return plainerr("error: tried to use indirection expression as statement");
                        }
                        if ast.child(0)?.text != "funcexpr"
                        {
                            return plainerr("error: tried to use unknown right-handed-expansion expression as statement");
                        }
                        let mut temp = Vec::new();
                        std::mem::swap(&mut temp, &mut ast.child_mut(0)?.children);
                        std::mem::swap(&mut temp, &mut ast.children);
                    }
                    while ast.children.len() > 2
                    {
                        let left = ASTNode{text: "funcexpr".to_string(), line: ast.child(0)?.line, position: ast.child(0)?.position, isparent: true, children: ast.children.drain(0..2).collect(), opdata: dummy_opdata()};
                        ast.children.insert(0, left);
                    }
                }
                "arrayref" =>
                {
                    while ast.children.len() > 2
                    {
                        let left = ASTNode{text: ast.text.clone(), line: ast.child(0)?.line, position: ast.child(0)?.position, isparent: true, children: ast.children.drain(0..2).collect(), opdata: dummy_opdata()};
                        ast.children.insert(0, left);
                    }
                }
                "rhunexpr" =>
                {
                    while ast.children.len() > 2
                    {
                        let left = ASTNode{text: ast.text.clone(), line: ast.child(0)?.line, position: ast.child(0)?.position, isparent: true, children: ast.children.drain(0..2).collect(), opdata: dummy_opdata()};
                        ast.children.insert(0, left);
                    }
                    
                    if ast.children.len() != 2
                    {
                        return plainerr("internal error: transformed rhunexpr doesn't have exactly two children");
                    }
                    if ast.child(1)?.children.len() != 1
                    {
                        return plainerr("internal error: right child of transformed rhunexpr doesn't have exactly one child");
                    }
                    
                    if ast.child(1)?.child(0)?.text == "funcargs"
                    {
                        ast.text = "funcexpr".to_string();
                        let mut temp = dummy_astnode();
                        std::mem::swap(&mut temp, ast.child_mut(1)?.child_mut(0)?);
                        std::mem::swap(&mut temp, ast.child_mut(1)?);
                    }
                    else if ast.child(1)?.child(0)?.text == "arrayindex"
                    {
                        ast.text = "arrayexpr".to_string();
                        let mut temp = dummy_astnode();
                        std::mem::swap(&mut temp, ast.child_mut(1)?.child_mut(0)?);
                        std::mem::swap(&mut temp, ast.child_mut(1)?);
                    }
                    else if ast.child(1)?.child(0)?.text == "indirection"
                    {
                        ast.text = "indirection".to_string();
                        let mut temp = dummy_astnode();
                        std::mem::swap(&mut temp, ast.child_mut(1)?.child_mut(0)?.child_mut(1)?);
                        std::mem::swap(&mut temp, ast.child_mut(1)?);
                    }
                    else
                    {
                        return plainerr("internal error: rhunexpr doesn't contain funcargs | arrayindex | indirection");
                    }
                }
                "ifcondition" | "whilecondition" | "withstatement" =>
                {
                    if ast.children.len() < 5
                    {
                        return plainerr("internal error: if/while/with loop doesn't have at least 5 children (this includes its parens) (it should have a token, paren, expr, paren, block)");
                    }
                    ast.children.remove(3); // )
                    ast.children.remove(1); // (
                }
                "foreach" =>
                {
                    if ast.children.len() != 7
                    {
                        return plainerr("internal error: foreach loop doesn't have exactly 7 children (this includes its parens) (it should have a token, paren, expr, paren, block)");
                    }
                    ast.children.remove(5); // )
                    ast.children.remove(3); // "in"
                    ast.children.remove(1); // (
                }
                "switch" =>
                {
                    ast.children.pop(); // }
                    ast.children.remove(4); // {
                    ast.children.remove(3); // )
                    ast.children.remove(1); // (
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
    
    fn verify_ast(&self, ast : &ASTNode) -> Result<(), String>
    {
        if ast.isparent
        {
            if ast.text == "objdef"
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
            
            if matches!(ast.text.as_str(), "funccall" | "funcexpr" | "arrayref")
               && ast.children.len() != 2
            {
                return plainerr("broken ast node");
            }
            
            for child in &ast.children
            {
                self.verify_ast(&child)?;
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
                if let Some(error) = latesterror
                {
                    let mut expected : Vec<String> = error.expected.into_iter().map(|x| x.to_string()).collect();
                    expected.sort();
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
                        let position = token.position;
                        if let Some(line) = lines.get(linenum-1)
                        {
                            println!("context:\n{}\n{}^", line, " ".repeat(position));
                        }
                        else
                        {
                            println!("internal error: failed to grab context text for parse error");
                        }
                    }
                    else
                    {
                        println!("internal error: failed to grab context info for parse error");
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
                    println!("verifying AST...");
                }
                self.verify_ast(&ast)?;
                
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
