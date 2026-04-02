from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
AUTHOR = "gaohongshun"
DATE = "2026-04-02"
SKIP_DIRS = {
    ".git",
    ".next",
    "node_modules",
    "target",
    "dist",
    "out",
    ".turbo",
}
RUST_SUFFIXES = {".rs"}
TS_SUFFIXES = {".ts", ".tsx", ".mjs"}
PS_SUFFIXES = {".ps1"}

RUST_FN_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\b")
TS_FN_RE = re.compile(r"^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_][A-Za-z0-9_]*)\b")
TS_ARROW_RE = re.compile(
    r"^\s*(?:export\s+)?const\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:async\s*)?(?:<[^=]+>\s*)?\("
)
PS_FN_RE = re.compile(r"^\s*function\s+([A-Za-z_][A-Za-z0-9_-]*)\b")
PS_PARAM_RE = re.compile(r"\$([A-Za-z_][A-Za-z0-9_]*)")


def detect_newline(text: str) -> str:
    return "\r\n" if "\r\n" in text else "\n"


def split_keepends(text: str) -> list[str]:
    return text.splitlines(keepends=True)


def leading_ws(line: str) -> str:
    return line[: len(line) - len(line.lstrip())]


def strip_line_ending(line: str) -> str:
    return line.rstrip("\r\n")


def prev_non_empty(lines: list[str], index: int) -> str | None:
    i = index - 1
    while i >= 0:
        value = strip_line_ending(lines[i]).strip()
        if value:
            return value
        i -= 1
    return None


def has_existing_comment(lines: list[str], index: int, lang: str) -> bool:
    prev = prev_non_empty(lines, index)
    if prev is None:
        return False
    if lang == "rust":
        return prev.startswith("///") or prev.startswith("//!") or prev.startswith("//")
    if lang == "ts":
        return (
            prev.startswith("/**")
            or prev.startswith("*/")
            or prev.startswith("*")
            or prev.startswith("//")
            or prev.startswith("/*")
        )
    if lang == "ps1":
        return prev.startswith("<#") or prev.startswith("#>") or prev.startswith("#")
    return False


def collect_signature(lines: list[str], start: int) -> tuple[str, int]:
    parts: list[str] = []
    paren_depth = 0
    saw_open = False
    i = start
    while i < len(lines):
        line = strip_line_ending(lines[i])
        parts.append(line.strip())
        for char in line:
            if char == "(":
                paren_depth += 1
                saw_open = True
            elif char == ")":
                paren_depth = max(paren_depth - 1, 0)
        joined = " ".join(parts)
        if saw_open and paren_depth == 0 and ("{" in line or "=>" in line or line.strip().endswith(";")):
            return joined, i
        if saw_open and paren_depth == 0 and i > start:
            next_index = i + 1
            if next_index >= len(lines):
                return joined, i
            next_stripped = strip_line_ending(lines[next_index]).strip()
            if not next_stripped.startswith("->") and next_stripped != "where":
                return joined, i
        i += 1
    return " ".join(parts), len(lines) - 1


def text_inside_first_parens(signature: str) -> str:
    start = signature.find("(")
    if start < 0:
        return ""
    depth = 0
    for index in range(start, len(signature)):
        char = signature[index]
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                return signature[start + 1 : index]
    return ""


def split_top_level(value: str) -> list[str]:
    if not value.strip():
        return []
    items: list[str] = []
    current: list[str] = []
    stack: list[str] = []
    pairs = {"(": ")", "[": "]", "{": "}", "<": ">"}
    closing = {v: k for k, v in pairs.items()}
    for char in value:
        if char == "," and not stack:
            item = "".join(current).strip()
            if item:
                items.append(item)
            current = []
            continue
        current.append(char)
        if char in pairs:
            stack.append(char)
        elif char in closing and stack and stack[-1] == closing[char]:
            stack.pop()
    item = "".join(current).strip()
    if item:
        items.append(item)
    return items


def normalize_param_name(raw: str, lang: str) -> str | None:
    value = raw.strip()
    if not value:
        return None
    if lang == "rust":
        value = re.sub(r"^mut\s+", "", value)
        if value in {"self", "&self", "&mut self", "mut self"}:
            return "self"
        if ":" in value:
            value = value.split(":", 1)[0].strip()
        value = value.lstrip("&").strip()
        value = re.sub(r"^mut\s+", "", value)
        return value or None
    if lang == "ts":
        value = value.split("=", 1)[0].strip()
        value = value.lstrip("...").strip()
        if ":" in value:
            value = value.split(":", 1)[0].strip()
        value = re.sub(r"^(public|private|protected|readonly)\s+", "", value)
        if value.startswith("{") or value.startswith("["):
            return "params"
        return value or None
    return None


def extract_params(signature: str, lang: str) -> list[str]:
    raw_params = text_inside_first_parens(signature)
    params: list[str] = []
    for item in split_top_level(raw_params):
        name = normalize_param_name(item, lang)
        if name:
            params.append(name)
    return params


def extract_ps_params(lines: list[str], start: int) -> list[str]:
    params: list[str] = []
    i = start + 1
    while i < len(lines):
        stripped = strip_line_ending(lines[i]).strip()
        if not stripped:
            i += 1
            continue
        if stripped.startswith("param("):
            while i < len(lines):
                current = strip_line_ending(lines[i])
                params.extend(PS_PARAM_RE.findall(current))
                if ")" in current:
                    return dedupe(params)
                i += 1
        break
    return dedupe(params)


def dedupe(items: list[str]) -> list[str]:
    seen: set[str] = set()
    ordered: list[str] = []
    for item in items:
        if item not in seen:
            seen.add(item)
            ordered.append(item)
    return ordered


def build_return_text(signature: str, lang: str) -> str:
    if lang == "rust":
        tail = signature.split(")", 1)[1] if ")" in signature else ""
        return "无" if "->" not in tail else "返回函数执行结果"
    if lang == "ts":
        return "返回函数执行结果"
    return "返回函数执行结果"


def build_comment(indent: str, name: str, params: list[str], return_text: str, lang: str, newline: str) -> str:
    if lang == "rust":
        lines = [
            f"{indent}/// 函数 `{name}`",
            f"{indent}///",
            f"{indent}/// 作者: {AUTHOR}",
            f"{indent}///",
            f"{indent}/// 时间: {DATE}",
            f"{indent}///",
            f"{indent}/// # 参数",
        ]
        if params:
            lines.extend(f"{indent}/// - {param}: 参数 {param}" for param in params)
        else:
            lines.append(f"{indent}/// 无")
        lines.extend(
            [
                f"{indent}///",
                f"{indent}/// # 返回",
                f"{indent}/// {return_text}",
                "",
            ]
        )
        return newline.join(lines)
    if lang == "ts":
        lines = [
            f"{indent}/**",
            f"{indent} * 函数 `{name}`",
            f"{indent} *",
            f"{indent} * 作者: {AUTHOR}",
            f"{indent} *",
            f"{indent} * 时间: {DATE}",
            f"{indent} *",
            f"{indent} * # 参数",
        ]
        if params:
            lines.extend(f"{indent} * - {param}: 参数 {param}" for param in params)
        else:
            lines.append(f"{indent} * 无")
        lines.extend(
            [
                f"{indent} *",
                f"{indent} * # 返回",
                f"{indent} * {return_text}",
                f"{indent} */",
                "",
            ]
        )
        return newline.join(lines)
    lines = [
        f"{indent}<#",
        f"{indent}函数 `{name}`",
        f"{indent}",
        f"{indent}作者: {AUTHOR}",
        f"{indent}",
        f"{indent}时间: {DATE}",
        f"{indent}",
        f"{indent}# 参数",
    ]
    if params:
        lines.extend(f"{indent}- {param}: 参数 {param}" for param in params)
    else:
        lines.append(f"{indent}无")
    lines.extend(
        [
            f"{indent}",
            f"{indent}# 返回",
            f"{indent}{return_text}",
            f"{indent}#>",
            "",
        ]
    )
    return newline.join(lines)


def rust_insert_index(lines: list[str], fn_index: int) -> int:
    insert_at = fn_index
    i = fn_index - 1
    while i >= 0:
        stripped = strip_line_ending(lines[i]).strip()
        if re.match(r"^#\[[^\]]*\]$", stripped):
            insert_at = i
            i -= 1
            continue
        break
    return insert_at


def process_rust(path: Path, text: str) -> tuple[str, int]:
    newline = detect_newline(text)
    lines = split_keepends(text)
    insertions: dict[int, str] = {}
    i = 0
    while i < len(lines):
        line = strip_line_ending(lines[i])
        match = RUST_FN_RE.match(line)
        if not match:
            i += 1
            continue
        insert_at = rust_insert_index(lines, i)
        if has_existing_comment(lines, insert_at, "rust"):
            _, end = collect_signature(lines, i)
            i = end + 1
            continue
        signature, end = collect_signature(lines, i)
        name = match.group(1)
        params = extract_params(signature, "rust")
        comment = build_comment(leading_ws(lines[insert_at]), name, params, build_return_text(signature, "rust"), "rust", newline)
        insertions.setdefault(insert_at, comment)
        i = end + 1
    if not insertions:
        return text, 0
    output: list[str] = []
    count = 0
    for index, line in enumerate(lines):
        if index in insertions:
            output.append(insertions[index])
            count += 1
        output.append(line)
    return "".join(output), count


def process_ts(path: Path, text: str) -> tuple[str, int]:
    newline = detect_newline(text)
    lines = split_keepends(text)
    insertions: dict[int, str] = {}
    i = 0
    while i < len(lines):
        line = strip_line_ending(lines[i])
        match = TS_FN_RE.match(line) or TS_ARROW_RE.match(line)
        if not match:
            i += 1
            continue
        if has_existing_comment(lines, i, "ts"):
            _, end = collect_signature(lines, i)
            i = end + 1
            continue
        signature, end = collect_signature(lines, i)
        name = match.group(1)
        params = extract_params(signature, "ts")
        comment = build_comment(leading_ws(lines[i]), name, params, build_return_text(signature, "ts"), "ts", newline)
        insertions.setdefault(i, comment)
        i = end + 1
    if not insertions:
        return text, 0
    output: list[str] = []
    count = 0
    for index, line in enumerate(lines):
        if index in insertions:
            output.append(insertions[index])
            count += 1
        output.append(line)
    return "".join(output), count


def process_ps1(path: Path, text: str) -> tuple[str, int]:
    newline = detect_newline(text)
    lines = split_keepends(text)
    insertions: dict[int, str] = {}
    i = 0
    while i < len(lines):
        line = strip_line_ending(lines[i])
        match = PS_FN_RE.match(line)
        if not match:
            i += 1
            continue
        if has_existing_comment(lines, i, "ps1"):
            i += 1
            continue
        name = match.group(1)
        params = extract_ps_params(lines, i)
        comment = build_comment(leading_ws(lines[i]), name, params, build_return_text(line, "ps1"), "ps1", newline)
        insertions.setdefault(i, comment)
        i += 1
    if not insertions:
        return text, 0
    output: list[str] = []
    count = 0
    for index, line in enumerate(lines):
        if index in insertions:
            output.append(insertions[index])
            count += 1
        output.append(line)
    return "".join(output), count


def iter_target_files() -> list[Path]:
    files: list[Path] = []
    for path in ROOT.rglob("*"):
        if not path.is_file():
            continue
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        if path.name == "add_function_comments.py":
            continue
        suffix = path.suffix.lower()
        if suffix in RUST_SUFFIXES | TS_SUFFIXES | PS_SUFFIXES:
            files.append(path)
    return files


def process_file(path: Path) -> int:
    text = path.read_text(encoding="utf-8")
    suffix = path.suffix.lower()
    if suffix in RUST_SUFFIXES:
        updated, count = process_rust(path, text)
    elif suffix in TS_SUFFIXES:
        updated, count = process_ts(path, text)
    elif suffix in PS_SUFFIXES:
        updated, count = process_ps1(path, text)
    else:
        return 0
    if count > 0 and updated != text:
        path.write_text(updated, encoding="utf-8", newline="")
    return count


def main() -> None:
    file_count = 0
    comment_count = 0
    for path in iter_target_files():
        count = process_file(path)
        if count > 0:
            file_count += 1
            comment_count += count
    print(f"updated_files={file_count}")
    print(f"inserted_comments={comment_count}")


if __name__ == "__main__":
    main()
