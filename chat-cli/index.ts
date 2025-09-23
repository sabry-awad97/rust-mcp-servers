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
// Using readline for interactive menus instead of enquirer for simplicity
import fs from "fs";
import { cristal, fruit, pastel, rainbow, summer } from "gradient-string";
import * as readline from "node:readline/promises";
import { z } from "zod";

const terminal = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
});

const messages: ModelMessage[] = [];

// Interactive menu types and functions
interface ToolActionResult {
  action: "approve" | "edit" | "deny" | "details";
  editedArgs?: Record<string, any>;
}

async function showToolDetailsViewer(tool: EnhancedTool): Promise<void> {
  const details = [
    `${rainbow("🔧 Tool Details")}
`,
    `${cristal("Name:")} ${tool.originalName}`,
    `${cristal("Server:")} ${tool.serverName}`,
    `${cristal("Description:")} ${
      tool.description || "No description available"
    }`,
    `${cristal("Auto-approved:")} ${
      tool.autoApprove.includes(tool.originalName) ? "✅ Yes" : "❌ No"
    }`,
    `\n${summer("Input Schema:")}`,
    JSON.stringify(tool.inputSchema, null, 2),
  ].join("\n");

  console.log(
    boxen(details, {
      padding: 1,
      margin: 1,
      borderStyle: "round",
      borderColor: "blue",
    })
  );

  await terminal.question(pastel("Press Enter to continue..."));
}

async function editToolArguments(
  args: Record<string, any>
): Promise<Record<string, any>> {
  console.log(
    boxen(
      rainbow("📝 Edit Tool Arguments\n\n") +
        "Current arguments:\n" +
        JSON.stringify(args, null, 2) +
        "\n\nEnter new JSON or press Enter to keep current:",
      {
        padding: 1,
        margin: 1,
        borderStyle: "round",
        borderColor: "yellow",
      }
    )
  );

  const newArgsString = await terminal.question(
    summer("New arguments (JSON): ")
  );

  try {
    if (!newArgsString.trim()) {
      return args; // Keep original if empty
    }

    const newArgs = JSON.parse(newArgsString);
    console.log(
      boxen(
        rainbow("✅ Arguments Updated\n\n") + JSON.stringify(newArgs, null, 2),
        {
          padding: 1,
          margin: 1,
          borderStyle: "round",
          borderColor: "green",
        }
      )
    );
    return newArgs;
  } catch (error) {
    console.log(
      boxen(fruit("❌ Invalid JSON format\n\n") + "Using original arguments.", {
        padding: 1,
        margin: 1,
        borderStyle: "round",
        borderColor: "red",
      })
    );
    return args;
  }
}

async function showInteractiveToolMenu(
  tool: EnhancedTool,
  args: Record<string, any>
): Promise<ToolActionResult> {
  while (true) {
    console.log(
      boxen(
        rainbow("🛠️  Interactive Tool Menu\n\n") +
          cristal("Choose an action:\n") +
          summer("1. ✅ Approve & Execute\n") +
          summer("2. 📝 Edit Arguments\n") +
          summer("3. 🔍 View Tool Details\n") +
          summer("4. ❌ Deny\n"),
        {
          padding: 1,
          margin: 1,
          borderStyle: "round",
          borderColor: "cyan",
        }
      )
    );

    try {
      const choice = await terminal.question(
        fruit("Enter your choice (1-4): ")
      );

      switch (choice.trim()) {
        case "1":
        case "approve":
        case "a":
          return { action: "approve" };

        case "2":
        case "edit":
        case "e": {
          const editedArgs = await editToolArguments(args);
          return { action: "edit", editedArgs };
        }

        case "3":
        case "details":
        case "d":
          await showToolDetailsViewer(tool);
          // Continue the loop to show menu again
          break;

        case "4":
        case "deny":
        case "n":
          return { action: "deny" };

        default:
          console.log(fruit("❌ Invalid choice. Please enter 1-4."));
          break;
      }
    } catch (error) {
      // User cancelled (Ctrl+C), treat as deny
      return { action: "deny" };
    }
  }
}

// Zod schemas for validation
const ServerConfigSchema = z.object({
  command: z.string().min(1, "Command cannot be empty"),
  args: z.array(z.string()).optional().default([]),
  env: z.record(z.string(), z.string()).optional(),
  disabled: z.boolean().optional().default(false),
  autoApprove: z.array(z.string()).optional().default([]),
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
  autoApprove: string[];
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
      autoApprove: (tool as any).autoApprove || [],
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

  // Second pass: create AI tools WITH approval-based execute function
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
        // Check if tool is auto-approved
        const isAutoApproved = tool.autoApprove.includes(tool.originalName);

        if (isAutoApproved) {
          console.log(
            boxen(
              rainbow("🔧 Auto-Approved Tool Call:\n\n") +
                `Tool: ${summer(tool.originalName)}\n` +
                `Server: ${cristal(tool.serverName)}\n` +
                `Description: ${tool.description || "No description"}\n\n` +
                `Arguments:\n${JSON.stringify(args, null, 2)}`,
              {
                padding: 1,
                margin: 1,
                borderStyle: "round",
                borderColor: "green",
              }
            )
          );
        } else {
          // Display tool call for approval
          console.log(
            boxen(
              rainbow("🔧 Tool Call Approval Required:\n\n") +
                `Tool: ${summer(tool.originalName)}\n` +
                `Server: ${cristal(tool.serverName)}\n` +
                `Description: ${tool.description || "No description"}\n\n` +
                `Arguments:\n${JSON.stringify(args, null, 2)}`,
              {
                padding: 1,
                margin: 1,
                borderStyle: "round",
                borderColor: "yellow",
              }
            )
          );
        }

        // Handle tool approval (skip if auto-approved)
        let approved = isAutoApproved;
        let finalArgs = args;

        if (!isAutoApproved) {
          const menuResult = await showInteractiveToolMenu(tool, args);

          switch (menuResult.action) {
            case "approve":
              approved = true;
              break;
            case "edit":
              approved = true;
              finalArgs = menuResult.editedArgs || args;
              break;
            case "deny":
            default:
              approved = false;
              break;
          }
        }

        if (approved) {
          try {
            console.log(`Executing ${tool.originalName}...`);
            const result = await serverClient.client.callTool({
              name: tool.originalName,
              arguments: finalArgs,
            });
            console.log(`✓ ${tool.originalName} completed`);

            // Display beautiful result immediately after execution
            const formattedResult = JSON.stringify(result, null, 2);

            console.log(
              boxen(rainbow("🎉 Tool Result:\n\n") + formattedResult, {
                padding: 1,
                margin: 1,
                borderStyle: "round",
                borderColor: "green",
              })
            );

            return result;
          } catch (error) {
            console.error(`✗ ${tool.originalName} failed:`, error);
            throw error;
          }
        } else {
          console.log(fruit("❌ Tool call denied by user"));
          // Return a structured response that informs the AI about the denial
          return {
            content: [
              {
                type: "text",
                text: `Tool call "${tool.originalName}" was denied by the user. The user chose not to execute this tool. Please continue without using this tool or ask the user for an alternative approach.`,
              },
            ],
            isError: false,
          };
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
    try {
      // Validate server configuration with Zod
      const config = ServerConfigSchema.parse(rawConfig);

      // Skip disabled servers
      if (config.disabled) {
        console.log(`⏭️  Skipping ${serverName} server (disabled)`);
        continue;
      }

      console.log(`Connecting to ${serverName} server...`);

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

      // Add server name and auto-approve config to each tool for tracking
      const toolsWithServer = tools.map((tool) => ({
        ...tool,
        serverName,
        autoApprove: config.autoApprove || [],
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
        system: `You are an AI assistant with access to multiple Model Context Protocol (MCP) servers that provide various tools and capabilities. Here's how the system works:

## Tool Approval System
- Before any tool is executed, the user must explicitly approve it
- When you want to use a tool, the user will see a detailed approval prompt showing:
  - Tool name and description
  - Which server it comes from  
  - The exact arguments you want to pass
- The user can approve (y/yes) or deny (n/no) each tool call
- If denied, you'll receive a message explaining the user chose not to execute that tool

## Available Tool Categories
You have access to tools from multiple servers:
- **Filesystem**: Read/write files, list directories, search files, manage file operations
- **Time**: Get current time, convert between timezones, time calculations  
- **Fetch**: Retrieve web content, fetch URLs, convert HTML to markdown

## CRITICAL: Always Explain Your Plan First
NEVER execute tools without first explaining your approach. For EVERY user request:

1. **Start with your plan**: "To [accomplish the task], I'll need to:"
2. **List the tools**: Explain which specific tools you'll use and why
3. **Explain the sequence**: Describe the order of operations
4. **Set expectations**: Tell the user what you're looking for
5. **Then execute**: Only after explaining, proceed with tool calls

## Example Response Pattern:
User: "Read my README.md file"
You: "To read your README.md file, I'll need to:
1. First use 'list_allowed_directories' to see what directories I can access
2. Then use 'read_text_file' to read the actual README.md content
Let me start by checking the available directories..."

## Best Practices
1. **Always Explain First**: Never use tools without explaining your plan
2. **Be Proactive**: Use available tools to solve problems, don't just ask for paths
3. **Handle Denials Gracefully**: If a tool is denied, suggest alternatives
4. **Minimize Tool Calls**: Only use tools when necessary
5. **Respect User Choices**: If denied, don't repeatedly ask for the same tool
6. **Be Helpful**: Offer alternative solutions when tools are unavailable

## Communication Style
- Always start responses by explaining your planned approach
- Be clear about which tools you'll use and why
- Acknowledge when users deny tool calls and explain alternatives
- Prioritize using available tools over asking users for information

Remember: ALWAYS explain your plan before executing any tools. Users want to see your reasoning process.`,
        messages,
        tools: aiTools,
        stopWhen: stepCountIs(20),
        prepareStep: async ({ steps, stepNumber, messages }) => {
          // Beautiful step logging
          if (stepNumber > 1) {
            // Analyze previous steps
            const previousStep = steps[steps.length - 1];
            const totalToolsExecuted = steps.reduce(
              (total, step) => total + (step.toolResults?.length || 0),
              0
            );

            let stepSummary = "";
            if (previousStep) {
              const toolCount = previousStep.toolResults?.length || 0;
              const lastReason = previousStep.finishReason;
              stepSummary = `\n${fruit(
                `Previous step: ${lastReason} (${toolCount} tools executed)`
              )}\n`;
            }

            console.log(
              boxen(
                rainbow(`🧠 AI Reasoning Step ${stepNumber}\n\n`) +
                  pastel(
                    "The AI is analyzing the conversation and deciding on next actions...\n"
                  ) +
                  stepSummary +
                  cristal(`Messages in context: ${messages.length}\n`) +
                  summer(`Total steps completed: ${steps.length}\n`) +
                  pastel(
                    `Total tools executed so far: ${totalToolsExecuted}\n`
                  ) +
                  summer("Evaluating available tools and planning approach..."),
                {
                  padding: 1,
                  margin: 1,
                  borderStyle: "round",
                  borderColor: "blue",
                  dimBorder: true,
                }
              )
            );
          }

          // Return default settings (no modifications)
          return {};
        },
        onStepFinish: async ({ toolResults, finishReason }) => {
          // Beautiful step completion logging
          const finishReasonEmoji: Record<string, string> = {
            stop: "✅",
            "tool-calls": "🛠️",
            length: "📏",
            "content-filter": "🚫",
            error: "❌",
            other: "❓",
            unknown: "❔",
          };

          console.log(
            boxen(
              rainbow(`📋 Step Completed\n\n`) +
                summer(
                  `Reason: ${finishReason} ${
                    finishReasonEmoji[finishReason] || "❔"
                  }\n`
                ) +
                (toolResults && toolResults.length > 0
                  ? pastel(`Tools executed: ${toolResults.length}`)
                  : pastel("No tools executed")),
              {
                padding: 1,
                margin: 1,
                borderStyle: "round",
                borderColor: "magenta",
                dimBorder: true,
              }
            )
          );
          // Tool results are now displayed immediately after execution
          // No need to display them again here
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
