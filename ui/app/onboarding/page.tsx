"use client";
import { useEffect, useMemo, useState } from "react";
import CopyButton from "../components/CopyButton";

type Provider = "openai" | "anthropic" | "gemini" | "azure";
type Lang     = "python" | "node" | "curl" | "langchain" | "env";

const PROVIDERS: { id: Provider; label: string; path: string }[] = [
  { id: "openai",    label: "OpenAI",    path: "/v1/openai/v1"    },
  { id: "anthropic", label: "Anthropic", path: "/v1/anthropic/v1" },
  { id: "gemini",    label: "Gemini",    path: "/v1/gemini"       },
  { id: "azure",     label: "Azure OpenAI", path: "/v1/azure"     },
];

function mask(k: string) {
  if (!k) return "";
  if (k.length <= 12) return "•".repeat(k.length);
  return k.slice(0, 6) + "••••••••" + k.slice(-4);
}

function snippet(lang: Lang, provider: Provider, gateway: string, key: string, agentId: string): string {
  const p = PROVIDERS.find(p => p.id === provider)!;
  const base = `${gateway}${p.path}`;
  const headers =
`x-agentsentry-key:   ${key}
x-agentsentry-agent: ${agentId}`;

  if (lang === "env") {
    if (provider === "openai") {
      return `# .env  (point the official OpenAI SDK at AgentSentry)
OPENAI_BASE_URL=${base}
OPENAI_API_KEY=sk-your-real-openai-key
# AgentSentry sees every request:
AGENTSENTRY_KEY=${key}
AGENTSENTRY_AGENT=${agentId}`;
    }
    if (provider === "anthropic") {
      return `ANTHROPIC_BASE_URL=${base}
ANTHROPIC_API_KEY=sk-ant-your-real-key
AGENTSENTRY_KEY=${key}
AGENTSENTRY_AGENT=${agentId}`;
    }
    return `# Set in your shell / deployment env
GATEWAY_BASE_URL=${base}
AGENTSENTRY_KEY=${key}
AGENTSENTRY_AGENT=${agentId}`;
  }

  if (lang === "curl") {
    if (provider === "anthropic") {
      return `curl ${base}/messages \\
  -H "content-type: application/json" \\
  -H "x-api-key: $ANTHROPIC_API_KEY" \\
  -H "anthropic-version: 2023-06-01" \\
  -H "x-agentsentry-key: ${key}" \\
  -H "x-agentsentry-agent: ${agentId}" \\
  -d '{"model":"claude-3-5-sonnet-latest","max_tokens":256,"messages":[{"role":"user","content":"hello"}]}'`;
    }
    if (provider === "gemini") {
      return `curl "${base}/v1beta/models/gemini-1.5-flash:generateContent?key=$GEMINI_API_KEY" \\
  -H "content-type: application/json" \\
  -H "x-agentsentry-key: ${key}" \\
  -H "x-agentsentry-agent: ${agentId}" \\
  -d '{"contents":[{"parts":[{"text":"hello"}]}]}'`;
    }
    if (provider === "azure") {
      return `curl "${base}/openai/deployments/$AZURE_DEPLOY/chat/completions?api-version=2024-08-01-preview" \\
  -H "content-type: application/json" \\
  -H "api-key: $AZURE_OPENAI_KEY" \\
  -H "x-agentsentry-key: ${key}" \\
  -H "x-agentsentry-agent: ${agentId}" \\
  -d '{"messages":[{"role":"user","content":"hello"}]}'`;
    }
    return `curl ${base}/chat/completions \\
  -H "content-type: application/json" \\
  -H "Authorization: Bearer $OPENAI_API_KEY" \\
  -H "x-agentsentry-key: ${key}" \\
  -H "x-agentsentry-agent: ${agentId}" \\
  -d '{"model":"gpt-4o-mini","messages":[{"role":"user","content":"hello"}]}'`;
  }

  if (lang === "python") {
    if (provider === "openai") {
      return `# pip install openai
from openai import OpenAI

client = OpenAI(
    base_url="${base}",
    api_key="sk-your-real-openai-key",
    default_headers={
        "x-agentsentry-key":   "${key}",
        "x-agentsentry-agent": "${agentId}",
    },
)
print(client.chat.completions.create(
    model="gpt-4o-mini",
    messages=[{"role":"user","content":"hello"}],
).choices[0].message.content)`;
    }
    if (provider === "anthropic") {
      return `# pip install anthropic
from anthropic import Anthropic

client = Anthropic(
    base_url="${base}",
    api_key="sk-ant-your-real-key",
    default_headers={
        "x-agentsentry-key":   "${key}",
        "x-agentsentry-agent": "${agentId}",
    },
)
print(client.messages.create(
    model="claude-3-5-sonnet-latest",
    max_tokens=256,
    messages=[{"role":"user","content":"hello"}],
).content[0].text)`;
    }
    return `# Build the URL yourself and add the AgentSentry headers
import httpx, os
HEADERS = {
    "x-agentsentry-key":   "${key}",
    "x-agentsentry-agent": "${agentId}",
}
r = httpx.post(
    "${base}/...",
    headers={**HEADERS, "content-type":"application/json"},
    json={...},
    timeout=60,
)
print(r.status_code, r.text)`;
  }

  if (lang === "node") {
    if (provider === "openai") {
      return `// npm i openai
import OpenAI from "openai";

const client = new OpenAI({
  baseURL: "${base}",
  apiKey: process.env.OPENAI_API_KEY,
  defaultHeaders: {
    "x-agentsentry-key":   "${key}",
    "x-agentsentry-agent": "${agentId}",
  },
});

const r = await client.chat.completions.create({
  model: "gpt-4o-mini",
  messages: [{ role: "user", content: "hello" }],
});
console.log(r.choices[0].message.content);`;
    }
    if (provider === "anthropic") {
      return `// npm i @anthropic-ai/sdk
import Anthropic from "@anthropic-ai/sdk";

const client = new Anthropic({
  baseURL: "${base}",
  apiKey: process.env.ANTHROPIC_API_KEY,
  defaultHeaders: {
    "x-agentsentry-key":   "${key}",
    "x-agentsentry-agent": "${agentId}",
  },
});

const r = await client.messages.create({
  model: "claude-3-5-sonnet-latest",
  max_tokens: 256,
  messages: [{ role: "user", content: "hello" }],
});
console.log(r.content[0]);`;
    }
    return `// generic fetch
const r = await fetch("${base}/...", {
  method: "POST",
  headers: {
    "content-type": "application/json",
    "x-agentsentry-key":   "${key}",
    "x-agentsentry-agent": "${agentId}",
  },
  body: JSON.stringify({ /* provider payload */ }),
});
console.log(await r.text());`;
  }

  // langchain
  if (provider === "openai") {
    return `# pip install langchain-openai
from langchain_openai import ChatOpenAI

llm = ChatOpenAI(
    model="gpt-4o-mini",
    base_url="${base}",
    api_key="sk-your-real-openai-key",
    default_headers={
        "x-agentsentry-key":   "${key}",
        "x-agentsentry-agent": "${agentId}",
    },
)
print(llm.invoke("hello"))`;
  }
  if (provider === "anthropic") {
    return `# pip install langchain-anthropic
from langchain_anthropic import ChatAnthropic

llm = ChatAnthropic(
    model="claude-3-5-sonnet-latest",
    anthropic_api_url="${base}",
    api_key="sk-ant-your-real-key",
    default_headers={
        "x-agentsentry-key":   "${key}",
        "x-agentsentry-agent": "${agentId}",
    },
)
print(llm.invoke("hello"))`;
  }
  return `# LangChain doesn't ship a wrapper for ${p.label} — use the raw HTTP example above.`;
}

export default function Onboarding() {
  const [gateway, setGateway] = useState("http://localhost:8080");
  const [key, setKey]         = useState("");
  const [agentId, setAgent]   = useState("support-bot");
  const [provider, setProv]   = useState<Provider>("openai");
  const [lang, setLang]       = useState<Lang>("python");
  const [revealed, setRev]    = useState(false);

  useEffect(() => {
    // Best-effort: pull persisted values from localStorage so the page is sticky.
    const g = localStorage.getItem("as.gw");
    const k = localStorage.getItem("as.key");
    const a = localStorage.getItem("as.agent");
    if (g) setGateway(g);
    if (k) setKey(k);
    if (a) setAgent(a);
    // Default the gateway URL to the current host if it looks like a hosted UI.
    if (!g && typeof window !== "undefined" && window.location.hostname !== "localhost") {
      setGateway(`${window.location.protocol}//${window.location.hostname}:8080`);
    }
  }, []);
  useEffect(() => { localStorage.setItem("as.gw", gateway); }, [gateway]);
  useEffect(() => { localStorage.setItem("as.key", key); },   [key]);
  useEffect(() => { localStorage.setItem("as.agent", agentId); }, [agentId]);

  const code = useMemo(() => snippet(lang, provider, gateway.replace(/\/+$/,""), key || "<TENANT_API_KEY>", agentId),
    [lang, provider, gateway, key, agentId]);

  return (
    <>
      <div className="page-head"><h2>Agent onboarding</h2></div>
      <p className="muted">
        Three things go on the agent side: <b>change the base URL</b>, <b>add two headers</b>,
        and <b>keep your real LLM key</b>. Pick a provider + language below and paste.
      </p>

      <div className="panel">
        <h3>1 · Connection</h3>
        <div className="form-grid">
          <label>
            <span>Gateway URL</span>
            <input className="input" value={gateway} onChange={e => setGateway(e.target.value)} placeholder="http://localhost:8080" />
          </label>
          <label>
            <span>Tenant API key</span>
            <div className="input-group">
              <input className="input" type={revealed ? "text" : "password"} value={key} onChange={e => setKey(e.target.value)} placeholder="sk_..." />
              <button className="btn btn-sm" onClick={() => setRev(r => !r)}>{revealed ? "Hide" : "Show"}</button>
              <CopyButton text={key} label="Copy key" />
            </div>
            <div className="muted small">stored locally only · {key ? mask(key) : "not set"}</div>
          </label>
          <label>
            <span>Agent ID</span>
            <input className="input" value={agentId} onChange={e => setAgent(e.target.value)} placeholder="support-bot" />
          </label>
        </div>
      </div>

      <div className="panel">
        <h3>2 · Pick provider + stack</h3>
        <div className="tabs">
          {PROVIDERS.map(p => (
            <button key={p.id} className={`tab ${p.id === provider ? "active" : ""}`} onClick={() => setProv(p.id)}>{p.label}</button>
          ))}
        </div>
        <div className="tabs">
          {(["python","node","curl","langchain","env"] as Lang[]).map(l => (
            <button key={l} className={`tab ${l === lang ? "active" : ""}`} onClick={() => setLang(l)}>{l}</button>
          ))}
        </div>
        <div className="code-block">
          <div className="code-head">
            <code className="muted small">{PROVIDERS.find(p => p.id === provider)!.label} · {lang}</code>
            <CopyButton text={code} />
          </div>
          <pre>{code}</pre>
        </div>
      </div>

      <div className="panel">
        <h3>3 · Verify</h3>
        <ol>
          <li>Run the snippet above against your gateway.</li>
          <li>Open <a href="/traces">Traces</a> — you should see a row with your <code>agent</code> id within ~2 seconds.</li>
          <li>Try sending a deny-triggering prompt (e.g. <code>&quot;the password is prod_db_password&quot;</code>) — you should get HTTP 403 and a <span className="badge badge-deny">deny</span> row.</li>
        </ol>
        <p className="muted small">
          Stuck? Check the gateway logs (<code>docker logs agentsentry-dev-gateway-1</code>) and the
          control plane (<code>docker logs agentsentry-dev-control-1</code>). The status pill in the
          sidebar shows live control-plane health.
        </p>
      </div>
    </>
  );
}
