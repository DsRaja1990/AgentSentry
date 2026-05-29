import Link from "next/link";
import { api, type Policy } from "@/lib/api";

export const dynamic = "force-dynamic";

export default async function Policies() {
  let items: Policy[] = [];
  let error: string | null = null;
  try { items = (await api.policies()).items ?? []; }
  catch (e: unknown) { error = e instanceof Error ? e.message : String(e); }

  return (
    <>
      <h2>Policies</h2>
      {error && <div className="error">{error}</div>}
      {items.length === 0 && !error && <p style={{color:"var(--muted)"}}>No policies defined.</p>}
      {items.length > 0 && (
        <table>
          <thead><tr><th>Name</th><th>Lang</th><th>Status</th><th>Ver</th><th>Updated</th></tr></thead>
          <tbody>
            {items.map(p => (
              <tr key={p.id}>
                <td><Link href={`/policies/${p.id}`}>{p.name}</Link></td>
                <td>{p.language}</td>
                <td><span className={`badge ${p.status === "enforced" ? "badge-deny" : "badge-default"}`}>{p.status}</span></td>
                <td>{p.version}</td>
                <td>{new Date(p.created_at).toLocaleString()}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </>
  );
}
