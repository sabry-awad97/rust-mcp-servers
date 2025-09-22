import "dotenv/config";

import { google } from "@ai-sdk/google";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import {
  jsonSchema,
  type ModelMessage,
  stepCountIs,
  streamText,
  type ToolSet,
} from "ai";
import boxen from "boxen";
import { Command } from "commander";
import fs from "fs";
import { cristal, fruit, pastel, rainbow, summer } from "gradient-string";
import * as readline from "node:readline/promises";
import { z } from "zod";

const terminal = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
});

const messages: ModelMessage[] = [];

// Zod schemas for validation
const ServerConfigSchema = z.object({
  command: z.string().min(1, "Command cannot be empty"),
  args: z.array(z.string()).optional().default([]),
  env: z.record(z.string(), z.string()).optional(),
});

const ServersConfigSchema = z.object({
  mcpServers: z.record(z.string(), ServerConfigSchema),
});

type ServersConfig = z.infer<typeof ServersConfigSchema>;

interface ServerClient {
  client: Client;
  serverName: string;
}

interface EnhancedTool {
  name: string;
  originalName: string;
  description?: string;
  inputSchema: unknown;
  serverName: string;
}

function createUniqueToolName(
  toolName: string,
  serverName: string,
  existingNames: Set<string>
): string {
  if (!existingNames.has(toolName)) {
    return toolName;
  }

  // If there's a conflict, prefix with server name
  const prefixedName = `${serverName}_${toolName}`;
  if (!existingNames.has(prefixedName)) {
    return prefixedName;
  }

  // If still conflicts, add a counter
  let counter = 1;
  let uniqueName = `${serverName}_${toolName}_${counter}`;
  while (existingNames.has(uniqueName)) {
    counter++;
    uniqueName = `${serverName}_${toolName}_${counter}`;
  }

  return uniqueName;
}

function mapMcpToolsToAiTools(
  serverClients: ServerClient[],
  allTools: Array<{
    name: string;
    description?: string;
    inputSchema: unknown;
    serverName: string;
  }>
): { aiTools: ToolSet; toolMapping: Map<string, EnhancedTool> } {
  const toolMapping = new Map<string, EnhancedTool>();
  const existingNames = new Set<string>();
  const conflicts: string[] = [];

  // First pass: identify conflicts and create unique names
  const enhancedTools: EnhancedTool[] = allTools.map((tool) => {
    const uniqueName = createUniqueToolName(
      tool.name,
      tool.serverName,
      existingNames
    );

    if (uniqueName !== tool.name) {
      conflicts.push(`${tool.name} → ${uniqueName} (from ${tool.serverName})`);
    }

    existingNames.add(uniqueName);

    const enhancedTool: EnhancedTool = {
      name: uniqueName,
      originalName: tool.name,
      description: tool.description,
      inputSchema: tool.inputSchema,
      serverName: tool.serverName,
    };

    toolMapping.set(uniqueName, enhancedTool);
    return enhancedTool;
  });

  // Log conflicts if any
  if (conflicts.length > 0) {
    console.log(
      boxen(
        rainbow("⚠️  Tool Name Conflicts Resolved:\n\n") + conflicts.join("\n"),
        {
          padding: 1,
          margin: 1,
          borderStyle: "round",
          borderColor: "yellow",
        }
      )
    );
  }

  // Second pass: create AI tools
  const aiTools = enhancedTools.reduce((acc, tool) => {
    const serverClient = serverClients.find(
      (sc) => sc.serverName === tool.serverName
    );
    if (!serverClient) {
      console.warn(`No client found for server: ${tool.serverName}`);
      return acc;
    }

    acc[tool.name] = {
      description: `${tool.description ?? "No description"} [from ${
        tool.serverName
      }]`,
      inputSchema: jsonSchema(tool.inputSchema),
      execute: async (args: Record<string, any>) => {
        console.log(`Executing ${tool.originalName} on ${tool.serverName}...`);
        try {
          const result = await serverClient.client.callTool({
            name: tool.originalName, // Use original name for the actual call
            arguments: args,
          });
          console.log(`✓ ${tool.originalName} completed`);
          return result;
        } catch (error) {
          console.error(`✗ ${tool.originalName} failed:`, error);
          throw error;
        }
      },
    };

    return acc;
  }, {} as ToolSet);

  return { aiTools, toolMapping };
}

async function connectToServers(
  serverConfig: ServersConfig["mcpServers"]
): Promise<{
  serverClients: ServerClient[];
  allTools: Array<{
    name: string;
    description?: string;
    inputSchema: unknown;
    serverName: string;
  }>;
}> {
  const serverClients: ServerClient[] = [];
  const allTools: Array<{
    name: string;
    description?: string;
    inputSchema: unknown;
    serverName: string;
  }> = [];

  console.log("🔌 Connecting to MCP servers...\n");

  for (const [serverName, rawConfig] of Object.entries(serverConfig)) {
    console.log(`Connecting to ${serverName} server...`);

    try {
      // Validate server configuration with Zod
      const config = ServerConfigSchema.parse(rawConfig);

      const transport = new StdioClientTransport({
        command: config.command,
        args: config.args,
        env: config.env,
      });

      const client = new Client({
        name: "fetch-cli-client",
        version: "1.0.0",
      });

      await client.connect(transport);

      let tools;
      try {
        const result = await client.listTools();
        tools = result.tools;

        // Fix schema compatibility issues
        tools = tools.map((tool, index) => {
          const schema = tool.inputSchema as any;

          // Fix missing or invalid type field
          if (!schema || typeof schema !== "object") {
            console.log(
              `⚠️  Fixing invalid schema for tool ${index} (${tool.name})`
            );
            return {
              ...tool,
              inputSchema: {
                type: "object",
                properties: {},
                additionalProperties: false,
              },
            };
          }

          if (schema.type !== "object") {
            console.log(
              `⚠️  Fixing schema type for tool ${index} (${tool.name}): ${schema.type} -> object`
            );
            return {
              ...tool,
              inputSchema: {
                ...schema,
                type: "object",
              },
            };
          }

          return tool;
        });
      } catch (toolError) {
        const errorMessage =
          toolError instanceof Error ? toolError.message : String(toolError);
        console.error(
          `✗ Failed to list tools for ${serverName}:`,
          errorMessage
        );
        continue;
      }

      serverClients.push({ client, serverName });

      // Add server name to each tool for tracking
      const toolsWithServer = tools.map((tool) => ({
        ...tool,
        serverName,
      }));

      allTools.push(...toolsWithServer);
      console.log(
        `✓ Connected to ${serverName} (${tools.length} tools available)`
      );
    } catch (error) {
      if (error instanceof z.ZodError) {
        console.error(
          `✗ Invalid configuration for ${serverName} server:`,
          error.issues
        );
      } else {
        console.error(`✗ Failed to connect to ${serverName} server:`, error);
      }
    }
  }

  return { serverClients, allTools };
}

async function loadMcpConfig(configPath: string): Promise<ServersConfig> {
  try {
    const configData = fs.readFileSync(configPath, "utf-8");
    const config = JSON.parse(configData);
    return ServersConfigSchema.parse(config);
  } catch (error) {
    console.error(
      `Failed to load MCP configuration from ${configPath}:`,
      error
    );
    // Fallback to empty config
    return {
      mcpServers: {},
    };
  }
}

async function main() {
  // Parse command line arguments
  const program = new Command();
  program
    .name("mcp-cli")
    .description("Multi-server MCP CLI Client")
    .version("1.0.0")
    .option(
      "-c, --config <path>",
      "Path to MCP configuration file",
      "./mcp-config.json"
    )
    .parse();

  const options = program.opts();
  const configPath = options.config;

  console.log(
    boxen(
      rainbow("🤖 MCP CLI Client") +
        "\n\n" +
        pastel("Connect to multiple Model Context Protocol servers") +
        "\n" +
        `Config: ${summer(configPath)}\n` +
        "Type 'help' for available commands or 'exit' to quit",
      {
        padding: 1,
        margin: 1,
        borderStyle: "round",
        borderColor: "cyan",
      }
    )
  );

  // Load MCP server configuration
  const servers = await loadMcpConfig(configPath);

  // Connect to all configured servers
  const { serverClients, allTools } = await connectToServers(
    servers.mcpServers
  );

  if (serverClients.length === 0) {
    console.error("❌ No servers connected successfully. Exiting...");
    process.exit(1);
  }

  // Map all tools from all servers to AI tools
  const { aiTools, toolMapping } = mapMcpToolsToAiTools(
    serverClients,
    allTools
  );

  const serverStats = serverClients.map((sc) => {
    const serverTools = Array.from(toolMapping.values()).filter(
      (t) => t.serverName === sc.serverName
    );
    return `${cristal(sc.serverName)}: ${summer(
      serverTools.length.toString()
    )} tools`;
  });

  console.log(
    boxen(
      rainbow("🛠️  Available Tools Summary\n\n") +
        serverStats.join("\n") +
        "\n\n" +
        "Use 'list-tools' to see all available tools",
      {
        padding: 1,
        margin: 1,
        borderStyle: "round",
        borderColor: "green",
      }
    )
  );

  // Add graceful shutdown handling
  process.on("SIGINT", () => {
    console.log("\n👋 Gracefully shutting down...");
    process.exit(0);
  });

  while (true) {
    try {
      const userInput = await terminal.question("You: ");

      // Handle special commands
      if (userInput.trim().toLowerCase() === "exit") {
        console.log(
          boxen(rainbow("👋 Goodbye!"), {
            padding: 1,
            margin: 1,
            borderStyle: "round",
            borderColor: "yellow",
          })
        );
        process.exit(0);
      }

      if (userInput.trim().toLowerCase() === "list-tools") {
        const toolsByServer = new Map<string, EnhancedTool[]>();

        for (const tool of toolMapping.values()) {
          if (!toolsByServer.has(tool.serverName)) {
            toolsByServer.set(tool.serverName, []);
          }
          toolsByServer.get(tool.serverName)!.push(tool);
        }

        let toolsOutput = rainbow("🔧 Available Tools\n\n");

        for (const [serverName, tools] of toolsByServer) {
          toolsOutput += cristal(`${serverName}:\n`);
          tools.forEach((tool) => {
            const displayName =
              tool.name !== tool.originalName
                ? `${summer(tool.name)} (${tool.originalName})`
                : summer(tool.name);
            toolsOutput += `  • ${displayName} - ${
              tool.description || "No description"
            }\n`;
          });
          toolsOutput += "\n";
        }

        console.log(
          boxen(toolsOutput.trim(), {
            padding: 1,
            margin: 1,
            borderStyle: "round",
            borderColor: "green",
          })
        );
        continue;
      }

      if (userInput.trim().toLowerCase() === "help") {
        console.log(
          boxen(
            rainbow("📚 Available Commands\n\n") +
              summer("help") +
              " - Show this help message\n" +
              summer("list-tools") +
              " - List all available tools\n" +
              summer("exit") +
              " - Exit the application",
            {
              padding: 1,
              margin: 1,
              borderStyle: "round",
              borderColor: "blue",
            }
          )
        );
        continue;
      }

      messages.push({ role: "user", content: userInput });

      const result = streamText({
        model: google("gemini-2.5-flash"),
        messages,
        tools: aiTools,
        stopWhen: stepCountIs(20),
        onStepFinish: async ({ toolResults }) => {
          if (toolResults.length) {
            // Format tool results with proper indentation for boxen
            const formattedResults = JSON.stringify(toolResults, null, 2)
              .split("\n")
              .map((line) => (line.length > 0 ? `  ${line}` : line))
              .join("\n");

            console.log(
              boxen(rainbow("🔧 Tool Results:\n") + formattedResults, {
                padding: 1,
                margin: 1,
                borderStyle: "round",
                borderColor: "green",
              })
            );
          }
        },
      });

      let fullResponse = "";
      process.stdout.write("\n" + pastel("Assistant: "));
      for await (const delta of result.textStream) {
        fullResponse += delta;
        process.stdout.write(delta);
      }
      process.stdout.write("\n\n");

      messages.push({ role: "assistant", content: fullResponse });
    } catch (error) {
      console.log(
        boxen(
          fruit("❌ An error occurred:\n\n") +
            String(error) +
            "\n\n" +
            summer(
              '💡 Try typing "help" for available commands or "exit" to quit.'
            ),
          {
            padding: 1,
            margin: 1,
            borderStyle: "round",
            borderColor: "red",
          }
        )
      );
    }
  }
}

main().catch(console.error);
