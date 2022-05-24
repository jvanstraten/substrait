import re

class Token:
    def __init__(self, span, cls, match):
        super().__init__()

        # Two-tuple of source locations, each in turn a (line, column) tuple.
        self.span = span

        # String representing what type of token this is.
        self.cls = cls

        # Regex match object for the token.
        self.match = match

    def __str__(self):
        if self.match:
            text = self.match.group(0)
            if len(text) > 50:
                text = ' ' + repr(text[:20]) + '..' + repr(text[-20:])
            else:
                text = ' ' + repr(text)
        else:
            text = ''

        return (
            f'{self.span[0][0]}:{self.span[0][1]}..'
            f'{self.span[1][0]}:{self.span[1][1]}:'
            f'{text} '
            f'({self.cls})'
        )


def tokenize(data):
    """Tokenizes a string into Tokens All characters will be made part of a
    token, so joining all matches yields exactly the original string."""
    tokens = dict(
        skip_header=re.compile(r"\(\*\*\*\* (.+?) \*\*\*\*\)"),
        docstr=re.compile(r"\(\*\*(.+?)\*\)", re.S),
        annot=re.compile(r"\(\*:(.+?)\*\)", re.S),
        skip_comment=re.compile(r"\(\*.+?\*\)", re.S),
        skip_space=re.compile(r"\s+"),
        rule=re.compile(r"<([a-zA-Z_][a-zA-Z_0-9.]*)>"),
        liter=re.compile(r'"((?:[^"\\]|\\.)*)"'),
        chset=re.compile(r'\[(.)-(.)\]'),
        gen=re.compile(r"::="),
        open=re.compile(r"\("),
        close=re.compile(r"\)"),
        alter=re.compile(r"\|"),
        multi=re.compile(r"[*+?]"),
        semicol=re.compile(r";"),
    )

    line = 1
    col = 1

    while data:
        longest_length = 0
        longest_match = None
        longest_cls = None
        for cls, regex in tokens.items():
            match = regex.match(data)
            if match:
                length = len(match.group(0))
            else:
                length = 0
            if length > longest_length:
                longest_length = length
                longest_match = match
                longest_cls = cls
        if not longest_match:
            raise ValueError(f'Failed to tokenize near "{data[:30]}"')
        start = (line, col)
        for c in data[:longest_length]:
            if c == '\n':
                line += 1
                col = 1
            else:
                col += 1
        end = (line, col)
        span = (start, end)
        data = data[longest_length:]
        yield Token(span, longest_cls, longest_match)


def strip_spacing(tokens, prefix='skip_'):
    """Strips all tokens whose type starts with the given prefix."""
    for token in tokens:
        if not token.cls.startswith(prefix):
            yield token


class ParseError:
    def __init__(self, msg):
        super().__init__()
        self.msg = msg

    def __str__(self):
        return self.msg

    def __bool__(self):
        return False


def single_token_from_ebnf(tokens, cls):
    """Parser function that matches a single token of the specified cls."""
    if tokens[0].cls != cls:
        return ParseError(f'{tokens[0]}: expected {cls}'), tokens
    return tokens[0].match, tokens[1:]


def docstring_from_ebnf(tokens):
    """Parser function that matches a docstring."""
    docstr, tokens = single_token_from_ebnf(tokens, 'docstr')
    if isinstance(docstr, ParseError):
        return docstr, tokens
    lines = []
    for line in docstr.group(1).split('\n'):
        line = line.strip()
        if line.startswith('*'):
            line = line[1:]
            if line.startswith(' '):
                line = line[1:]
        lines.append(line)
    while lines and not lines[0]:
        lines = lines[1:]
    while lines and not lines[-1]:
        lines = lines[:-1]
    return '\n'.join(lines), tokens


def antlr_escape(literal):
    literal = literal.replace('\\', '\\\\')
    literal = literal.replace('\n', '\\n')
    literal = literal.replace('\r', '\\r')
    literal = literal.replace('\t', '\\t')
    literal = literal.replace('\b', '\\b')
    literal = literal.replace('\f', '\\f')
    literal = literal.replace("'", "\\'")
    return literal


def antlr_docstring(doc, indent):
    if not doc:
        return
    for line in doc.split('\n'):
        yield ' '*indent + '// ' + line


def camel_case(name):
    elements = name.split('_')
    return elements[0].lower() + ''.join(map(str.title, elements[1:]))


def title_case(name):
    return ''.join(map(str.title, name.split('_')))


class Pattern:
    """A matchable pattern."""

    def __init__(self):
        super().__init__()

    @staticmethod
    def from_ebnf(tokens):
        """Parses a single pattern or a parenthesized pattern."""
        init_tokens = tokens

        # Handle parenthesized patterns.
        if tokens[0].cls == 'open':
            tokens = tokens[1:]

            # Parse contents.
            pattern, tokens = Alters.from_ebnf(tokens)
            if isinstance(pattern, ParseError):
                return pattern, init_tokens

            # Expect close-paren.
            close, tokens = single_token_from_ebnf(tokens, 'close')
            if isinstance(close, ParseError):
                return close, init_tokens

            return pattern, tokens

        # Try literals.
        pattern, tokens = Literal.from_ebnf(init_tokens)
        if not isinstance(pattern, ParseError):
            return pattern, tokens

        # Try character sets.
        pattern, tokens = CharSet.from_ebnf(init_tokens)
        if not isinstance(pattern, ParseError):
            return pattern, tokens

        # Try non-terminal rule usage.
        pattern, tokens = NonTerminal.from_ebnf(init_tokens)
        if not isinstance(pattern, ParseError):
            return pattern, tokens

        return ParseError(f'{tokens[0]}: expected a pattern'), init_tokens

    def suggest_name(self):
        """Suggests a name for this pattern. Returns None if no reasonable name
        could be suggested."""
        return None

    def resolve(self, grammar):
        """Resolves rule references in this pattern using the given grammar."""
        pass

    def to_antlr(self):
        """Converts this pattern to ANTLR4 syntax."""
        raise NotImplementedError()
        yield None


class Literal(Pattern):
    """A pattern that matches a literal string case-insensitively."""

    def __init__(self, text):
        super().__init__()
        self.text = text

    @classmethod
    def from_ebnf(cls, tokens):
        """Parses a literal string."""
        match, tokens = single_token_from_ebnf(tokens, 'liter')
        if isinstance(match, ParseError):
            return match, tokens
        text = ''
        chars = iter(match.group(1))
        for char in chars:
            if char == '\\':
                char = next(chars)
                text += {
                    't': '\t',
                    'r': '\r',
                    'n': '\n',
                    'b': '\b',
                    'f': '\f',
                }.get(char, char)
            else:
                text += char
        return cls(text), tokens

    def suggest_name(self):
        parts = ['']
        for c in self.text:
            if not re.match(r'[a-zA-Z]', c):
                if parts[-1]:
                    parts.append('')
            else:
                parts[-1] += c
        if not parts[-1]:
            del parts[-1]
        return '_'.join(parts)

    def to_antlr(self):
        parts = ['']
        for c in self.text:
            if re.match(r'[a-zA-Z]', c):
                if not parts[-1]:
                    del parts[-1]
                else:
                    parts[-1] = "'" + parts[-1] + "'"
                parts.append(f'{c.upper()}')
                parts.append('')
            else:
                parts[-1] += antlr_escape(c)
        if len(parts) > 1 and not parts[-1]:
            del parts[-1]
        else:
            parts[-1] = "'" + parts[-1] + "'"
        if len(parts) == 1:
            yield parts[0]
        else:
            yield '( ' + ' '.join(parts) + ' )'


class CharSet(Pattern):
    """A pattern that matches a single character within an ordinal range."""

    def __init__(self, first, last):
        super().__init__()
        self.first = first
        self.last = last

    @classmethod
    def from_ebnf(cls, tokens):
        """Parses a character set/range."""
        match, tokens = single_token_from_ebnf(tokens, 'chset')
        if isinstance(match, ParseError):
            return match, tokens
        return cls(match.group(1), match.group(2)), tokens

    def to_antlr(self):
        yield f"'{antlr_escape(self.first)}'..'{antlr_escape(self.last)}'"


class NonTerminal(Pattern):
    """Represents usage of a rule within a pattern."""

    def __init__(self, name):
        super().__init__()

        # Name of the non-terminal pattern.
        self.name = name

        # The rule for matching the pattern, once resolved.
        self.rule = None

    @classmethod
    def from_ebnf(cls, tokens):
        """Parses a nonterminal rule reference."""
        match, tokens = single_token_from_ebnf(tokens, 'rule')
        if isinstance(match, ParseError):
            return match, tokens
        return cls(match.group(1)), tokens

    def suggest_name(self):
        return self.name

    def resolve(self, grammar):
        self.rule = grammar.rules.get(self.name, None)
        if self.rule is None:
            raise ValueError(f'rule {self.name} is not defined')

    def to_antlr(self):
        if self.rule.is_lexer_rule():
            yield title_case(self.name)
        else:
            yield camel_case(self.name)


class Multi(Pattern):
    """A pattern representing a specified number of copies of a single
    pattern."""

    def __init__(self, pattern, count):
        super().__init__()

        # The pattern.
        self.pattern = pattern

        # Must be '!' (one), '?' (zero or one), '+' (one or more), and
        # '*' (zero or more).
        self.count = count

    @classmethod
    def from_ebnf(cls, tokens):
        """Parses a single pattern with an optional multiplicity indicator."""
        init_tokens = tokens

        # Expect pattern.
        pattern, tokens = Pattern.from_ebnf(tokens)
        if isinstance(pattern, ParseError):
            return pattern, init_tokens

        # Parse optional character multiplicity.
        count, tokens = single_token_from_ebnf(tokens, 'multi')
        if isinstance(count, ParseError):
            count = '!'
        else:
            count = count.group(0)

        return cls(pattern, count), tokens

    def suggest_name(self):
        return self.pattern.suggest_name()

    def resolve(self, grammar):
        self.pattern.resolve(grammar)

    def to_antlr(self):
        contents = list(self.pattern.to_antlr())
        if len(contents) == 1:
            last = contents[0]
        else:
            yield '('
            yield from contents
            last = ')'
        if self.count != '!':
            last += self.count
        yield last


class Concat(Pattern):
    """A pattern representing a concatenation of patterns."""

    def __init__(self, patterns):
        super().__init__()
        self.patterns = patterns

    @classmethod
    def from_ebnf(cls, tokens):
        """Parses one or more concatenated patterns."""
        init_tokens = tokens

        # Expect first pattern.
        pattern, tokens = Multi.from_ebnf(tokens)
        if isinstance(pattern, ParseError):
            return pattern, init_tokens
        patterns = [pattern]

        # Match the rest of the pattern.
        while True:
            pattern, tokens = Multi.from_ebnf(tokens)
            if isinstance(pattern, ParseError):
                break
            patterns.append(pattern)

        return cls(patterns), tokens

    def suggest_name(self):
        if len(self.patterns) == 1:
            return self.patterns[0].suggest_name()
        return None

    def resolve(self, grammar):
        for pattern in self.patterns:
            pattern.resolve(grammar)

    def to_antlr(self):
        for pattern in self.patterns:
            yield from pattern.to_antlr()


class Alters(Pattern):
    """A pattern representing a number of alternative patterns."""

    def __init__(self, patterns):
        super().__init__()
        self.patterns = patterns

    @classmethod
    def from_ebnf(cls, tokens):
        """Parses one or more alternative patterns."""
        init_tokens = tokens

        # Expect first alternative.
        pattern, tokens = Concat.from_ebnf(tokens)
        if isinstance(pattern, ParseError):
            return pattern, init_tokens
        patterns = [pattern]

        # Match the rest of the alternatives.
        while tokens[0].cls == 'alter':
            tokens = tokens[1:]
            pattern, tokens = Concat.from_ebnf(tokens)
            if isinstance(pattern, ParseError):
                return pattern, init_tokens
            patterns.append(pattern)

        return cls(patterns), tokens

    def suggest_name(self):
        if len(self.patterns) == 1:
            return self.patterns[0].suggest_name()
        return None

    def resolve(self, grammar):
        for pattern in self.patterns:
            pattern.resolve(grammar)

    def to_antlr(self):
        yield from self.patterns[0].to_antlr()
        for pattern in self.patterns[1:]:
            yield '|'
            yield from pattern.to_antlr()


class Alter:
    """Toplevel alternative for a rule."""
    def __init__(self, pattern):
        super().__init__()

        # Docstring for the alter.
        self.doc = None

        # Name of the alter, if any has been set or generated yet.
        self.name = None

        # The Pattern for this alter.
        self.pattern = pattern

    @classmethod
    def from_ebnf(cls, tokens):
        """Parses a toplevel alternative for a rule."""
        init_tokens = tokens

        # Parse optional docstring.
        doc, tokens = docstring_from_ebnf(tokens)

        # Parse optional annotation.
        annot, tokens = single_token_from_ebnf(tokens, 'annot')

        # Expect pattern.
        pattern, tokens = Concat.from_ebnf(tokens)
        if isinstance(pattern, ParseError):
            return pattern, init_tokens

        # Construct Alter.
        alter = cls(pattern)
        alter.doc = doc
        if not isinstance(annot, ParseError):
            alter.name = annot.group(1)

        return alter, tokens

    def suggest_name(self):
        return self.pattern.suggest_name()

    def resolve(self, grammar):
        self.pattern.resolve(grammar)

    def to_antlr(self):
        yield from self.pattern.to_antlr()


class Rule:
    def __init__(self, name):
        super().__init__()

        # Docstring for the rule.
        self.doc = None

        # Name of the rule.
        self.name = name

        # Rule that this collapses into, if any.
        self.collapse_into = None

        # One of:
        #  - 'parse': normal parser rule.
        #  - 'text': normal lexer rule.
        #  - 'skip': lexer rule ignored in the AST (for whitespace).
        #  - 'frag': fragment of a lexer rule.
        self.mode = 'parse'

        # List of toplevel Alters.
        self.alters = []

        # Named variants to lists of equivalent Alters indices.
        self.variants = {}

    @classmethod
    def from_ebnf(cls, tokens):
        """Parses a Rule."""
        init_tokens = tokens

        # Parse name.
        rule, tokens = NonTerminal.from_ebnf(tokens)
        if isinstance(rule, ParseError):
            return rule, init_tokens
        rule = cls(rule.name)

        # Parse optional docstring (this comes after the name because we move
        # docstring tokens one token forward as a preprocessing step).
        rule.doc, tokens = docstring_from_ebnf(tokens)

        # Parse optional annotation.
        annot, tokens = single_token_from_ebnf(tokens, 'annot')
        if not isinstance(annot, ParseError):
            args = annot.group(1).split(':')
            if len(args) == 2 and args[0] == 'collapse':
                rule.collapse_into = args[1]
            elif len(args) == 1 and args[0] in ['text', 'skip', 'frag']:
                rule.mode = args[0]
            else:
                raise ValueError(f'unknown annotation: {annot.group(0)}')

        # Expect ::= token.
        gen, tokens = single_token_from_ebnf(tokens, 'gen')
        if isinstance(gen, ParseError):
            return gen, init_tokens

        # Expect first alternative.
        alter, tokens = Alter.from_ebnf(tokens)
        if isinstance(alter, ParseError):
            return alter, init_tokens
        rule.alters.append(alter)

        # Match the rest of the alternatives.
        while tokens[0].cls == 'alter':
            tokens = tokens[1:]
            alter, tokens = Alter.from_ebnf(tokens)
            if isinstance(alter, ParseError):
                return alter, init_tokens
            rule.alters.append(alter)

        # Generate unique names for all alters that were not explicitly named.
        names = set()
        for alter in rule.alters:
            if alter.name is not None:
                names.add(alter.name)
        for alter in rule.alters:
            if alter.name is None:
                name = alter.suggest_name()
                if not name:
                    name = 'anon'
                if name in names:
                    i = 2
                    while (unique := f'{name}_{i}') in names:
                        i += 1
                    name = unique
                alter.name = name
                names.add(name)

        # Generate variants.
        for idx, alter in enumerate(rule.alters):
            if alter.name in rule.variants:
                rule.variants[alter.name].append(idx)
            else:
                rule.variants[alter.name] = [idx]

        # Expect semicolon.
        semicol, tokens = single_token_from_ebnf(tokens, 'semicol')
        if isinstance(semicol, ParseError):
            return semicol, init_tokens

        return rule, tokens

    def resolve(self, grammar):
        for alter in self.alters:
            alter.resolve(grammar)

    def is_lexer_rule(self):
        return self.mode != 'parse'

    def to_antlr(self):
        yield from antlr_docstring(self.doc, 0)
        if not self.is_lexer_rule():
            yield camel_case(self.name)
        elif self.mode == 'frag':
            yield 'fragment ' + title_case(self.name)
        else:
            yield title_case(self.name)
        for idx, alter in enumerate(self.alters):
            if alter.doc:
                yield ''
            yield from antlr_docstring(alter.doc, 2)
            alter_str = ' '.join(alter.to_antlr())
            if not self.is_lexer_rule():
                alter_str += ' #' + title_case(f'{self.name}_{alter.name}')
            if idx == 0:
                yield f'  : {alter_str}'
            else:
                yield f'  | {alter_str}'
        yield '  ;'
        yield ''


class Grammar:
    def __init__(self):
        super().__init__()
        self.rules = {}

    @classmethod
    def from_ebnf(cls, text):
        """Constructs a grammar from a string representing an EBNF file."""
        tokens = list(strip_spacing(tokenize(text)))

        # It's easier to parse docstrings when they're moved one token ahead.
        for i in reversed(range(len(tokens) - 1)):
            if tokens[i].cls == 'docstr':
                tokens[i], tokens[i+1] = tokens[i+1], tokens[i]

        # Add an EOF sentinel token that nothing matches.
        tokens.append(Token((tokens[-1].span[1], tokens[-1].span[1]), 'eof', None))

        # Parse the grammar.
        grammar = Grammar()
        while tokens:
            if tokens[0].cls == 'eof':
                break
            rule, tokens = Rule.from_ebnf(tokens)
            if isinstance(rule, ParseError):
                raise ValueError(rule.msg)
            grammar.rules[rule.name] = rule

        # Resolve rules.
        for rule in grammar.rules.values():
            rule.resolve(grammar)

        return grammar

    def to_antlr(self, name):
        """Converts the grammar to ANTLR4 syntax."""
        def generator():
            yield f'grammar {name};'
            yield ''
            for rule in self.rules.values():
                yield from rule.to_antlr()
        return '\n'.join(generator()) + '\n'


with open('type-expressions.ebnf', 'r', encoding='utf-8') as fil:
    print(Grammar.from_ebnf(fil.read()).to_antlr('banana'))
    #for x in strip_spacing(tokenize(fil.read())):
        #print(x)
