"""Small Rust lexical masks for source ownership tests.

The masks preserve byte positions and newlines, so declaration spans still
refer to the original source. They are not a Rust parser: a malformed literal
or comment fails closed rather than pretending the remaining text is code.
"""

import re


def _blank(source: str) -> str:
    return "".join("\n" if char == "\n" else " " for char in source)


def rust_mask(source: str, *, literals: bool) -> str:
    """Mask nested comments, and optionally string/character literals."""
    result = list(source)
    index = 0
    while index < len(source):
        if source.startswith("//", index):
            end = source.find("\n", index)
            end = len(source) if end < 0 else end
            result[index:end] = _blank(source[index:end])
            index = end
            continue
        if source.startswith("/*", index):
            start = index
            depth = 1
            index += 2
            while index < len(source) and depth:
                if source.startswith("/*", index):
                    depth += 1
                    index += 2
                elif source.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            if depth:
                raise ValueError("unterminated Rust block comment")
            result[start:index] = _blank(source[start:index])
            continue
        raw = re.match(r'(?:br|cr|r)(?P<hashes>#*)"', source[index:])
        if raw and (index == 0 or not (source[index - 1].isalnum() or source[index - 1] == "_")):
            terminator = '"' + raw.group("hashes")
            end = source.find(terminator, index + raw.end())
            if end < 0:
                raise ValueError("unterminated Rust raw string")
            end += len(terminator)
            if literals:
                result[index:end] = _blank(source[index:end])
            index = end
            continue
        prefixed_string = source.startswith(('b"', 'c"'), index)
        if prefixed_string or source[index] == '"':
            start = index
            index += 2 if prefixed_string else 1
            while index < len(source):
                if source[index] == "\\":
                    index += 2
                elif source[index] == '"':
                    index += 1
                    break
                else:
                    index += 1
            else:
                raise ValueError("unterminated Rust string")
            if literals:
                result[start:index] = _blank(source[start:index])
            continue
        byte_char = source.startswith("b'", index)
        char_start = index + (1 if byte_char else 0)
        if source[char_start] == "'":
            end = char_start + 1
            while end < len(source) and source[end] != "\n":
                if source[end] == "\\":
                    end += 2
                elif source[end] == "'":
                    end += 1
                    break
                elif source[end].isspace():
                    break
                else:
                    end += 1
            if end <= len(source) and source[end - 1] == "'" and end > char_start + 2:
                if literals:
                    result[index:end] = _blank(source[index:end])
                index = end
                continue
            if byte_char:
                raise ValueError("unterminated Rust byte character")
        index += 1
    return "".join(result)


def rust_code_tokens(source: str) -> list[str]:
    """Return identifiers and punctuation outside comments and literals."""
    return re.findall(r"[A-Za-z_][A-Za-z_0-9]*|::|[^\s]", rust_mask(source, literals=True))


def _attribute_end(code: str, start: int) -> int | None:
    if not code.startswith("#[", start):
        return None
    stack = []
    matching = {"[": "]", "(": ")", "{": "}"}
    for index in range(start + 1, len(code)):
        char = code[index]
        if char in matching:
            stack.append(matching[char])
        elif char in "])}":
            if not stack or stack.pop() != char:
                return None
            if not stack:
                return index + 1
    return None


def outer_attributes(source: str, start: int) -> list[str]:
    """Return contiguous outer attributes before an item, nearest first.

    Literal contents do not participate in delimiter matching. Comments and
    whitespace between attributes and the item do not end the prelude.
    """
    prefix = source[:start]
    code = rust_mask(prefix, literals=True)
    comments = rust_mask(prefix, literals=False)
    cursor = len(code.rstrip())
    attrs = []
    while cursor:
        candidate = code.rfind("#[", 0, cursor)
        while candidate >= 0 and _attribute_end(code, candidate) != cursor:
            candidate = code.rfind("#[", 0, candidate)
        if candidate < 0:
            break
        attrs.append(re.sub(r"\s+", "", comments[candidate:cursor]))
        cursor = len(code[:candidate].rstrip())
    return attrs


def has_cfg_attribute(attrs: list[str]) -> bool:
    return any(attr.startswith(("#[cfg(", "#[cfg_attr(")) for attr in attrs)


def brace_depth(source: str, start: int) -> int:
    """Count live Rust braces before a declaration, ignoring literal decoys."""
    depth = 0
    for token in rust_code_tokens(source[:start]):
        if token == "{":
            depth += 1
        elif token == "}":
            depth -= 1
            if depth < 0:
                raise ValueError("unbalanced Rust braces")
    return depth
