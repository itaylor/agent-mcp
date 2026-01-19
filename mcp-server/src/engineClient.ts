import { spawn, ChildProcess } from "node:child_process";
import { randomUUID } from "node:crypto";

export type EngineError = {
  code: string;
  message: string;
  details?: unknown;
};

export type EngineResponse<T = unknown> =
  | { id: string; ok: true; result: T }
  | { id: string; ok: false; error: EngineError };

type Pending = {
  resolve: (v: any) => void;
  reject: (e: any) => void;
  timeout?: NodeJS.Timeout;
};

export class RustEngineClient {
  private cmd: string;
  private args: string[];
  private proc: ChildProcess | null = null;

  private pending = new Map<string, Pending>();
  private stdoutBuf = "";

  constructor(opts: { cmd: string; args: string[] }) {
    this.cmd = opts.cmd;
    this.args = opts.args;
  }

  start() {
    if (this.proc) return;

    const p = spawn(this.cmd, this.args, {
      stdio: ["pipe", "pipe", "inherit"],
      windowsHide: true,
    });

    this.proc = p;

    p.stdout.setEncoding("utf8");
    p.stdout.on("data", (chunk: string) => this.onStdout(chunk));

    p.on("exit", (code, signal) => {
      const err = new Error(
        `Rust engine exited: code=${code} signal=${signal}`,
      );
      // Reject all in-flight requests
      for (const [id, pend] of this.pending.entries()) {
        pend.timeout && clearTimeout(pend.timeout);
        pend.reject(err);
        this.pending.delete(id);
      }
      this.proc = null;
    });

    p.on("error", (e) => {
      // spawn error
      const err = new Error(`Failed to start Rust engine: ${String(e)}`);
      for (const [id, pend] of this.pending.entries()) {
        pend.timeout && clearTimeout(pend.timeout);
        pend.reject(err);
        this.pending.delete(id);
      }
      this.proc = null;
    });
  }

  stop() {
    if (!this.proc) return;
    this.proc.kill();
    this.proc = null;
  }

  private ensureStarted() {
    if (!this.proc) this.start();
    if (!this.proc) throw new Error("Rust engine is not running.");
  }

  private onStdout(chunk: string) {
    this.stdoutBuf += chunk;

    while (true) {
      const nl = this.stdoutBuf.indexOf("\n");
      if (nl < 0) break;

      const line = this.stdoutBuf.slice(0, nl).trim();
      this.stdoutBuf = this.stdoutBuf.slice(nl + 1);

      if (!line) continue;

      let msg: EngineResponse;
      try {
        msg = JSON.parse(line);
      } catch {
        // Ignore malformed output; you can tighten this to fail hard.
        continue;
      }

      const pend = this.pending.get(msg.id);
      if (!pend) continue;

      pend.timeout && clearTimeout(pend.timeout);
      this.pending.delete(msg.id);

      if (msg.ok) pend.resolve(msg.result);
      else
        pend.reject(
          Object.assign(new Error(msg.error.message), {
            engineError: msg.error,
          }),
        );
    }
  }

  /**
   * Generic call.
   * `op` must match the Rust OpName strings: "list_dir" | "search_text" | ...
   */
  async call<T>(
    op: string,
    args: unknown,
    opts?: { timeoutMs?: number },
  ): Promise<T> {
    this.ensureStarted();
    const p = this.proc!;
    const id = randomUUID();

    const payload = { id, op, args };
    const line = JSON.stringify(payload) + "\n";

    const timeoutMs = opts?.timeoutMs ?? 30_000;

    return await new Promise<T>((resolve, reject) => {
      const pend: Pending = { resolve, reject };

      if (timeoutMs > 0) {
        pend.timeout = setTimeout(() => {
          this.pending.delete(id);
          reject(
            new Error(
              `Rust engine request timed out after ${timeoutMs}ms (op=${op})`,
            ),
          );
        }, timeoutMs);
      }

      this.pending.set(id, pend);

      if (!p.stdin) {
        pend.timeout && clearTimeout(pend.timeout);
        this.pending.delete(id);
        reject(new Error("Engine process stdin is not available"));
        return;
      }

      p.stdin.write(line, "utf8", (err) => {
        if (err) {
          pend.timeout && clearTimeout(pend.timeout);
          this.pending.delete(id);
          reject(err);
        }
      });
    });
  }

  // Convenience wrappers (optional)
  listDir(args: any) {
    return this.call("list_dir", args);
  }
  searchText(args: any) {
    return this.call("search_text", args, { timeoutMs: 60_000 });
  }
  readExcerpt(args: any) {
    return this.call("read_excerpt", args);
  }
  exploreCode(args: any) {
    return this.call("explore_code", args);
  }
  applyPatch(args: any) {
    return this.call("apply_patch", args, { timeoutMs: 60_000 });
  }
}
