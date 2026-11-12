// A stand-in for systemd's socket activation of the picture worker
// (ops/central-picture.socket with Accept=yes and MaxConnections=2,
// ops/central-picture@.service with StandardInput/Output=socket), for a
// local run against a real Central only (e2e-real/README.md): it listens on
// a Unix socket (mode 0600) and, for each connection, starts the command
// after `--` with the connection as its standard input and output, at most
// two at a time (a third connection is closed, as systemd refuses it).
// Loopback and this user only; nothing here is for production.
//
//   node e2e-real/picture-socket.ts --socket <absolute path> -- <command> [args…]
//
// The command is the worker itself (`central-picture-worker --ffmpeg <ffmpeg>`)
// or a container that runs it (`docker run -i --rm … central-picture-worker
// --ffmpeg /usr/bin/ffmpeg`), for an ffmpeg with libaom where this machine
// has none.
import { spawn } from "node:child_process";
import { chmodSync, rmSync } from "node:fs";
import { createServer } from "node:net";

const at = process.argv.indexOf("--socket");
const path = at > 0 ? (process.argv[at + 1] ?? "") : "";
const dash = process.argv.indexOf("--");
const command = dash > 0 ? process.argv.slice(dash + 1) : [];
if (!path.startsWith("/") || command.length === 0) {
  process.stderr.write("picture-socket.ts needs --socket <absolute path> -- <command> [args…]\n");
  process.exit(2);
}
const MAX = 2;
let running = 0;

rmSync(path, { force: true });
const server = createServer({ pauseOnConnect: true }, (connection) => {
  if (running >= MAX) {
    connection.destroy();
    return;
  }
  running += 1;
  // only what the command needs to be found and, for docker, to reach its daemon
  const env = { PATH: process.env["PATH"] ?? "/usr/bin:/bin", HOME: process.env["HOME"] ?? "/" };
  const child = spawn(command[0] as string, command.slice(1), { stdio: [connection, connection, "ignore"], env });
  let done = false;
  const end = () => {
    if (done) return;
    done = true;
    running -= 1;
    connection.destroy();
  };
  child.on("exit", end);
  child.on("error", end);
});
server.listen(path, () => {
  chmodSync(path, 0o600);
  process.stdout.write(`picture socket on ${path}, ${MAX} at a time\n`);
});
const stop = () => server.close(() => process.exit(0));
process.on("SIGTERM", stop);
process.on("SIGINT", stop);
