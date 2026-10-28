interface Env {
    r2: R2Bucket;
    assets: Fetcher;
}

export default {
    async fetch(request: Request, env: Env): Promise<Response> {
        const url = new URL(request.url);
        const key = url.pathname.slice(1);

        if (
            (request.method === "GET" || request.method === "HEAD") &&
            key.startsWith("pupu-") &&
            key.endsWith("_bg.wasm")
        ) {
            const object = await env.r2.get(key);

            if (!object) {
                return new Response("WASM not found", { status: 404 });
            }

            const headers = new Headers();
            object.writeHttpMetadata(headers);
            headers.set("Content-Type", "application/wasm");
            headers.set("Content-Length", String(object.size));
            headers.set("Cache-Control", "public, max-age=31536000, immutable");
            headers.set("ETag", object.httpEtag);

            return new Response(
                request.method === "HEAD" ? null : object.body,
                { headers },
            );
        }

        return env.assets.fetch(request);
    },
} satisfies ExportedHandler<Env>;
