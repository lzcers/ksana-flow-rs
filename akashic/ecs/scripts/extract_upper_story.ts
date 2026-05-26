#!/usr/bin/env bun

import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

declare const process: {
  argv: string[];
  cwd(): string;
  stdout: { write(chunk: string): void };
  exitCode?: number;
};

function extractAssistantSegments(text: string): string {
  const lines = text.split(/\r?\n/);
  const segments: string[] = [];
  let current: string[] | null = null;

  for (const line of lines) {
    const trimmed = line.trim();

    if (trimmed === "assistant:") {
      if (current !== null) {
        const segment = current.join("\n").trim();
        if (segment) {
          segments.push(segment);
        }
      }
      current = [];
      continue;
    }

    if (trimmed === "user:") {
      if (current !== null) {
        const segment = current.join("\n").trim();
        if (segment) {
          segments.push(segment);
        }
      }
      current = null;
      continue;
    }

    if (current !== null) {
      current.push(line);
    }
  }

  if (current !== null) {
    const segment = current.join("\n").trim();
    if (segment) {
      segments.push(segment);
    }
  }

  return segments.length > 0 ? `${segments.join("\n\n")}\n` : "";
}

function printUsage(scriptName: string): void {
  console.log(
    [
      "抽取对话日志中 assistant: 到下一个 user: 之间的文本，并拼接为完整故事。",
      "",
      `用法: ${scriptName} [input] [-o output]`,
      "",
      "参数:",
      "  input         输入文件路径，默认使用 akashic-ecs/upper_narrator_context.txt",
      "  -o, --output  可选的输出文件路径；不提供时打印到标准输出",
      "  -h, --help    显示帮助",
    ].join("\n"),
  );
}

type ParsedArgs = {
  inputPath: string;
  outputPath?: string;
  help: boolean;
};

function parseArgs(argv: string[], defaultInputPath: string): ParsedArgs {
  let inputPath = defaultInputPath;
  let outputPath: string | undefined;
  let inputConsumed = false;
  let help = false;

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "-h" || arg === "--help") {
      help = true;
      continue;
    }

    if (arg === "-o" || arg === "--output") {
      const next = argv[index + 1];
      if (!next) {
        throw new Error("缺少输出文件路径，请在 -o/--output 后提供路径。");
      }
      outputPath = next;
      index += 1;
      continue;
    }

    if (arg.startsWith("-")) {
      throw new Error(`未知参数: ${arg}`);
    }

    if (inputConsumed) {
      throw new Error(`多余参数: ${arg}`);
    }

    inputPath = arg;
    inputConsumed = true;
  }

  return { inputPath, outputPath, help };
}

async function main(): Promise<void> {
  const scriptFile = fileURLToPath(import.meta.url);
  const scriptDir = path.dirname(scriptFile);
  const defaultInputPath = path.resolve(scriptDir, "..", "upper_narrator_context.txt");
  const scriptName = path.relative(process.cwd(), scriptFile) || scriptFile;

  const { inputPath, outputPath, help } = parseArgs(
    process.argv.slice(2),
    defaultInputPath,
  );

  if (help) {
    printUsage(scriptName);
    return;
  }

  const resolvedInputPath = path.resolve(process.cwd(), inputPath);
  const story = extractAssistantSegments(
    await readFile(resolvedInputPath, "utf8"),
  );

  if (outputPath) {
    const resolvedOutputPath = path.resolve(process.cwd(), outputPath);
    await mkdir(path.dirname(resolvedOutputPath), { recursive: true });
    await writeFile(resolvedOutputPath, story, "utf8");
    console.log(`已写入: ${resolvedOutputPath}`);
    return;
  }

  process.stdout.write(story);
}

main().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`提取失败: ${message}`);
  process.exitCode = 1;
});
