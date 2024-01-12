import fastify from "fastify";

const server = fastify({ logger: true });

server.get("/", async () => ({ message: "Hello, World!" }));

server.listen({ port: 3000, host: "0.0.0.0" });
