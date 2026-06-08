import { createQrCodeMatrix } from './qrCode';

interface ShareCardImageInput {
  summary: string;
  gameUrl: string;
  eyebrow: string;
  title: string;
  sessionLabel: string;
}

const CARD_WIDTH = 1200;
const CARD_HEIGHT = 1500;
const CARD_PADDING = 72;

function drawRoundRect(
  context: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number,
) {
  context.beginPath();
  context.moveTo(x + radius, y);
  context.lineTo(x + width - radius, y);
  context.quadraticCurveTo(x + width, y, x + width, y + radius);
  context.lineTo(x + width, y + height - radius);
  context.quadraticCurveTo(x + width, y + height, x + width - radius, y + height);
  context.lineTo(x + radius, y + height);
  context.quadraticCurveTo(x, y + height, x, y + height - radius);
  context.lineTo(x, y + radius);
  context.quadraticCurveTo(x, y, x + radius, y);
  context.closePath();
}

function fillRoundRect(
  context: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number,
) {
  drawRoundRect(context, x, y, width, height, radius);
  context.fill();
}

function strokeRoundRect(
  context: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number,
) {
  drawRoundRect(context, x, y, width, height, radius);
  context.stroke();
}

function drawWrappedText(
  context: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  maxWidth: number,
  lineHeight: number,
  maxLines: number,
) {
  const normalized = text.trim() || '命运尚未留下可供摘录的回响。';
  const paragraphs = normalized.split(/\n+/);
  const lines: string[] = [];

  paragraphs.forEach((paragraph) => {
    let line = '';
    Array.from(paragraph).forEach((char) => {
      const nextLine = `${line}${char}`;
      if (line && context.measureText(nextLine).width > maxWidth) {
        lines.push(line);
        line = char;
      } else {
        line = nextLine;
      }
    });
    if (line) {
      lines.push(line);
    }
  });

  const visibleLines = lines.slice(0, maxLines);
  if (lines.length > maxLines && visibleLines.length > 0) {
    let lastLine = visibleLines[visibleLines.length - 1];
    while (lastLine.length > 1 && context.measureText(`${lastLine}...`).width > maxWidth) {
      lastLine = lastLine.slice(0, -1);
    }
    visibleLines[visibleLines.length - 1] = `${lastLine}...`;
  }

  visibleLines.forEach((line, index) => {
    context.fillText(line, x, y + (index * lineHeight));
  });
}

function drawQrCode(
  context: CanvasRenderingContext2D,
  gameUrl: string,
  x: number,
  y: number,
  size: number,
) {
  const qr = createQrCodeMatrix(gameUrl);
  const quietZone = 4;
  const moduleCount = qr.size + (quietZone * 2);
  const moduleSize = size / moduleCount;

  context.fillStyle = '#ffffff';
  fillRoundRect(context, x, y, size, size, 28);
  context.fillStyle = '#0b1220';

  qr.modules.forEach((row, rowIndex) => {
    row.forEach((isDark, columnIndex) => {
      if (!isDark) {
        return;
      }
      context.fillRect(
        x + ((columnIndex + quietZone) * moduleSize),
        y + ((rowIndex + quietZone) * moduleSize),
        Math.ceil(moduleSize),
        Math.ceil(moduleSize),
      );
    });
  });
}

function canvasToBlob(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (blob) {
        resolve(blob);
      } else {
        reject(new Error('分享卡片图片生成失败。'));
      }
    }, 'image/png');
  });
}

function downloadBlob(blob: Blob, filename: string) {
  const link = document.createElement('a');
  const url = URL.createObjectURL(blob);
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}

export async function downloadStoryShareCardImage(input: ShareCardImageInput): Promise<void> {
  const canvas = document.createElement('canvas');
  canvas.width = CARD_WIDTH;
  canvas.height = CARD_HEIGHT;

  const context = canvas.getContext('2d');
  if (!context) {
    throw new Error('当前浏览器不支持生成分享卡片。');
  }

  const background = context.createLinearGradient(0, 0, CARD_WIDTH, CARD_HEIGHT);
  background.addColorStop(0, '#0c1728');
  background.addColorStop(0.5, '#081020');
  background.addColorStop(1, '#10111d');
  context.fillStyle = background;
  context.fillRect(0, 0, CARD_WIDTH, CARD_HEIGHT);

  const glow = context.createRadialGradient(260, 170, 20, 260, 170, 430);
  glow.addColorStop(0, 'rgba(97,190,183,0.28)');
  glow.addColorStop(1, 'rgba(97,190,183,0)');
  context.fillStyle = glow;
  context.fillRect(0, 0, CARD_WIDTH, CARD_HEIGHT);

  const goldGlow = context.createRadialGradient(980, 170, 40, 980, 170, 380);
  goldGlow.addColorStop(0, 'rgba(232,204,130,0.22)');
  goldGlow.addColorStop(1, 'rgba(232,204,130,0)');
  context.fillStyle = goldGlow;
  context.fillRect(0, 0, CARD_WIDTH, CARD_HEIGHT);

  context.strokeStyle = 'rgba(216,193,143,0.46)';
  context.lineWidth = 2;
  strokeRoundRect(context, 38, 38, CARD_WIDTH - 76, CARD_HEIGHT - 76, 34);

  context.fillStyle = 'rgba(216,193,143,0.10)';
  fillRoundRect(context, CARD_PADDING, CARD_PADDING, 290, 54, 27);
  context.fillStyle = '#e6d1a2';
  context.font = '600 22px Inter, system-ui, sans-serif';
  context.letterSpacing = '4px';
  context.fillText(input.eyebrow.toUpperCase(), CARD_PADDING + 26, CARD_PADDING + 36);
  context.letterSpacing = '0px';

  context.fillStyle = 'rgba(207,244,248,0.75)';
  context.font = '600 24px Inter, system-ui, sans-serif';
  context.fillText(input.sessionLabel, CARD_PADDING, 210);

  context.fillStyle = '#f4ecd8';
  context.font = '500 62px Inter, "PingFang SC", "Microsoft YaHei", sans-serif';
  drawWrappedText(context, input.title, CARD_PADDING, 298, CARD_WIDTH - (CARD_PADDING * 2), 76, 2);

  context.fillStyle = 'rgba(8,14,26,0.44)';
  fillRoundRect(context, CARD_PADDING, 475, CARD_WIDTH - (CARD_PADDING * 2), 540, 30);
  context.strokeStyle = 'rgba(216,193,143,0.26)';
  context.lineWidth = 2;
  strokeRoundRect(context, CARD_PADDING, 475, CARD_WIDTH - (CARD_PADDING * 2), 540, 30);

  context.strokeStyle = 'rgba(216,193,143,0.62)';
  context.beginPath();
  context.moveTo(CARD_PADDING + 42, 555);
  context.lineTo(CARD_WIDTH - CARD_PADDING - 42, 555);
  context.stroke();

  context.fillStyle = 'rgba(230,209,162,0.88)';
  context.font = '600 22px Inter, system-ui, sans-serif';
  context.fillText('SUMMARY', CARD_PADDING + 42, 535);

  context.fillStyle = 'rgba(243,234,216,0.92)';
  context.font = '400 33px Inter, "PingFang SC", "Microsoft YaHei", sans-serif';
  drawWrappedText(context, input.summary, CARD_PADDING + 42, 625, CARD_WIDTH - (CARD_PADDING * 2) - 84, 58, 6);

  const qrSize = 240;
  const qrX = CARD_WIDTH - CARD_PADDING - qrSize;
  const qrY = 1090;
  drawQrCode(context, input.gameUrl, qrX, qrY, qrSize);

  context.fillStyle = 'rgba(207,244,248,0.74)';
  context.font = '600 22px Inter, "PingFang SC", "Microsoft YaHei", sans-serif';
  context.fillText('扫码复制独立分支', CARD_PADDING, 1148);

  context.fillStyle = 'rgba(233,222,200,0.82)';
  context.font = '400 30px Inter, "PingFang SC", "Microsoft YaHei", sans-serif';
  drawWrappedText(
    context,
    '带着这段摘要开启自己的分支，把下一轮选择亲手推向结局。',
    CARD_PADDING,
    1210,
    CARD_WIDTH - (CARD_PADDING * 3) - qrSize,
    48,
    3,
  );

  context.fillStyle = 'rgba(143,152,171,0.9)';
  context.font = '500 22px Inter, system-ui, sans-serif';
  context.fillText('AKASHIC SHARE CARD', CARD_PADDING, CARD_HEIGHT - 104);

  context.fillStyle = 'rgba(216,193,143,0.94)';
  context.font = '500 24px Inter, "PingFang SC", "Microsoft YaHei", sans-serif';
  context.fillText('分支复制链接已附在右侧二维码中', CARD_PADDING, CARD_HEIGHT - 66);

  const blob = await canvasToBlob(canvas);
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  downloadBlob(blob, `akashic-share-card-${timestamp}.png`);
}
