import { callCodexRpc } from "@/lib/server/codex-rpc-proxy";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

type ExportAccountFile = {
  fileName?: string;
  content?: string;
};

type AccountExportDataResult = {
  totalAccounts?: number;
  exported?: number;
  skippedMissingToken?: number;
  files?: ExportAccountFile[];
};

function pad2(value: number) {
  return String(value).padStart(2, "0");
}

function buildExportFileName(now = new Date()) {
  const year = now.getFullYear();
  const month = pad2(now.getMonth() + 1);
  const day = pad2(now.getDate());
  const hours = pad2(now.getHours());
  const minutes = pad2(now.getMinutes());
  const seconds = pad2(now.getSeconds());
  return `codexmanager-accounts-export-${year}${month}${day}-${hours}${minutes}${seconds}.json`;
}

function buildBrowserExportContent(files: ExportAccountFile[]) {
  const items = files.map((file, index) => {
    const content = String(file.content || "").trim();
    if (!content) {
      throw new Error(`第 ${index + 1} 条账号导出内容为空`);
    }

    try {
      return JSON.parse(content) as unknown;
    } catch (error) {
      const message = error instanceof Error && error.message.trim() ? error.message : "invalid JSON";
      throw new Error(`第 ${index + 1} 条账号导出内容无效：${message}`);
    }
  });

  return JSON.stringify(items, null, 2);
}

function buildProbePayload(result: AccountExportDataResult, fileName: string) {
  return {
    ok: true,
    fileName,
    totalAccounts: Number(result.totalAccounts || 0),
    exported: Number(result.exported || 0),
    skippedMissingToken: Number(result.skippedMissingToken || 0),
    fileCount: Array.isArray(result.files) ? result.files.length : 0,
  };
}

export async function GET(request: Request) {
  const url = new URL(request.url);
  const probeOnly = url.searchParams.get("probe") === "1";

  try {
    const result = await callCodexRpc<AccountExportDataResult>("account/exportData", undefined, request.signal);
    const fileName = buildExportFileName();

    if (probeOnly) {
      return Response.json(buildProbePayload(result, fileName), {
        headers: {
          "cache-control": "no-store",
        },
      });
    }

    const files = Array.isArray(result.files) ? result.files : [];
    const content = buildBrowserExportContent(files);
    return new Response(content, {
      status: 200,
      headers: {
        "content-type": "application/json; charset=utf-8",
        "content-disposition": `attachment; filename="${fileName}"`,
        "cache-control": "no-store",
      },
    });
  } catch (error) {
    const message = error instanceof Error && error.message.trim() ? error.message : "账号导出失败";
    if (probeOnly) {
      return Response.json(
        {
          ok: false,
          error: message,
        },
        {
          status: 500,
          headers: {
            "cache-control": "no-store",
          },
        },
      );
    }

    return new Response(message, {
      status: 500,
      headers: {
        "content-type": "text/plain; charset=utf-8",
        "cache-control": "no-store",
      },
    });
  }
}
