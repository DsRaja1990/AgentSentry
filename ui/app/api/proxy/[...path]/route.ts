// Server-side proxy so the browser never sees the control-plane API key.
// /api/proxy/<path> -> CONTROL_API_URL/<path> with bearer attached.
import { NextRequest, NextResponse } from "next/server";

const BASE =
  process.env.CONTROL_API_URL ??
  (process.env.NODE_ENV === "production" ? "http://control:8081" : "http://localhost:8081");
const KEY  = process.env.CONTROL_API_KEY ?? "sk_dev_local_demo_key";

async function forward(req: NextRequest, path: string[]) {
  const url = new URL(req.url);
  const target = `${BASE}/${path.join("/")}${url.search}`;
  const init: RequestInit = {
    method: req.method,
    headers: {
      "authorization": `Bearer ${KEY}`,
      "content-type": req.headers.get("content-type") ?? "application/json",
    },
    cache: "no-store",
  };
  if (req.method !== "GET" && req.method !== "HEAD") {
    init.body = await req.text();
  }
  try {
    const r = await fetch(target, init);
    const body = await r.text();
    return new NextResponse(body, {
      status: r.status,
      headers: { "content-type": r.headers.get("content-type") ?? "application/json" },
    });
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    return NextResponse.json({ error: "control_unreachable", detail: msg }, { status: 502 });
  }
}

export async function GET   (req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) { return forward(req, (await ctx.params).path); }
export async function POST  (req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) { return forward(req, (await ctx.params).path); }
export async function PUT   (req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) { return forward(req, (await ctx.params).path); }
export async function PATCH (req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) { return forward(req, (await ctx.params).path); }
export async function DELETE(req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) { return forward(req, (await ctx.params).path); }
