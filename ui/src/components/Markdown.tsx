import React from "react";

function renderInline(text: string): React.ReactNode[] {
  const parts: React.ReactNode[] = [];
  // code spans
  const codeRegex = /`([^`]+)`/g;
  let last = 0;
  let m: RegExpExecArray | null;
  const nodes: { type: "text" | "code"; value: string; index: number }[] = [];
  while ((m = codeRegex.exec(text)) !== null) {
    if (m.index > last) nodes.push({ type: "text", value: text.slice(last, m.index), index: nodes.length });
    nodes.push({ type: "code", value: m[1], index: nodes.length });
    last = m.index + m[0].length;
  }
  if (last < text.length) nodes.push({ type: "text", value: text.slice(last), index: nodes.length });

  nodes.forEach((n, i) => {
    if (n.type === "code") {
      parts.push(<code key={i}>{n.value}</code>);
    } else {
      // bold
      const boldParts = n.value.split(/(\*\*[^*]+\*\*)/g).map((seg, j) => {
        if (seg.startsWith("**") && seg.endsWith("**")) {
          return <strong key={j}>{seg.slice(2, -2)}</strong>;
        }
        return seg;
      });
      parts.push(<React.Fragment key={i}>{boldParts}</React.Fragment>);
    }
  });
  return parts;
}

export default function Markdown({ text }: { text: string }) {
  const lines = text.split("\n");
  const blocks: React.ReactNode[] = [];
  let i = 0;
  let codeBuf: string[] | null = null;
  let codeLang = "";
  const flushCode = () => {
    if (codeBuf) {
      const code = codeBuf.join("\n");
      blocks.push(
        <pre key={blocks.length}>
          <code>{code}</code>
        </pre>,
      );
      codeBuf = null;
    }
  };
  const paraBuf: string[] = [];
  const flushPara = () => {
    if (paraBuf.length) {
      const t = paraBuf.join(" ");
      blocks.push(<p key={blocks.length}>{renderInline(t)}</p>);
      paraBuf.length = 0;
    }
  };

  while (i < lines.length) {
    const line = lines[i];
    const fence = line.match(/^```(\w*)/);
    if (fence) {
      if (codeBuf) {
        flushCode();
      } else {
        flushPara();
        codeBuf = [];
        codeLang = fence[1];
      }
      i++;
      continue;
    }
    if (codeBuf) {
      codeBuf.push(line);
      i++;
      continue;
    }
    if (line.trim() === "") {
      flushPara();
      i++;
      continue;
    }
    if (/^#{1,3}\s/.test(line)) {
      flushPara();
      const level = line.match(/^#+/)![0].length;
      const txt = line.replace(/^#+\s*/, "");
      const Tag = (`h${Math.min(level + 1, 4)}` as any);
      blocks.push(<Tag key={blocks.length}>{renderInline(txt)}</Tag>);
      i++;
      continue;
    }
    if (/^[-*]\s/.test(line)) {
      flushPara();
      const items: string[] = [];
      while (i < lines.length && /^[-*]\s/.test(lines[i])) {
        items.push(lines[i].replace(/^[-*]\s/, ""));
        i++;
      }
      blocks.push(
        <ul key={blocks.length}>
          {items.map((it, j) => (
            <li key={j}>{renderInline(it)}</li>
          ))}
        </ul>,
      );
      continue;
    }
    paraBuf.push(line.trim());
    i++;
  }
  flushCode();
  flushPara();
  return <div className="message-body prose prose-sm max-w-none dark:prose-invert">{blocks}</div>;
}