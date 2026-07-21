import fs from "node:fs";
import net from "node:net";
import tls from "node:tls";

const [listen, target, certFile, keyFile, protocol] = process.argv.slice(2);
if (!listen || !target || !certFile || !keyFile || !["http1", "h2"].includes(protocol)) {
    throw new Error("usage: tls-shim.mjs LISTEN TARGET CERT KEY http1|h2");
}
const [listenHost, listenPort] = listen.split(":");
const [targetHost, targetPort] = target.split(":");

tls.createServer(
    {
        cert: fs.readFileSync(certFile),
        key: fs.readFileSync(keyFile),
        ALPNProtocols: protocol === "h2" ? ["h2"] : ["http/1.1"],
    },
    (client) => {
        const upstream = net.connect(Number(targetPort), targetHost);
        client.on("error", () => upstream.destroy());
        client.on("close", () => upstream.destroy());
        upstream.on("error", () => client.destroy());
        upstream.on("close", () => client.destroy());
        client.pipe(upstream);
        upstream.pipe(client);
    },
).listen(Number(listenPort), listenHost);
