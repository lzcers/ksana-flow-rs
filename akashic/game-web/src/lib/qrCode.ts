export interface QrCodeMatrix {
  size: number;
  modules: boolean[][];
}

const QR_VERSION = 5;
const QR_SIZE = 17 + (QR_VERSION * 4);
const DATA_CODEWORDS = 108;
const ECC_CODEWORDS = 26;
const MAX_BYTE_LENGTH = 106;
const FORMAT_MASK = 0x5412;
const FORMAT_POLYNOMIAL = 0x537;
const PRIMITIVE_POLYNOMIAL = 0x11d;

const encoder = new TextEncoder();

function appendBits(target: number[], value: number, length: number) {
  for (let i = length - 1; i >= 0; i -= 1) {
    target.push((value >>> i) & 1);
  }
}

function buildDataCodewords(input: string): number[] {
  const bytes = encoder.encode(input);
  if (bytes.length > MAX_BYTE_LENGTH) {
    throw new Error('分享链接过长，无法生成二维码。');
  }

  const bits: number[] = [];
  appendBits(bits, 0b0100, 4);
  appendBits(bits, bytes.length, 8);
  bytes.forEach((byte) => appendBits(bits, byte, 8));

  const capacityBits = DATA_CODEWORDS * 8;
  appendBits(bits, 0, Math.min(4, capacityBits - bits.length));
  while (bits.length % 8 !== 0) {
    bits.push(0);
  }

  const codewords: number[] = [];
  for (let i = 0; i < bits.length; i += 8) {
    let codeword = 0;
    for (let bit = 0; bit < 8; bit += 1) {
      codeword = (codeword << 1) | bits[i + bit];
    }
    codewords.push(codeword);
  }

  for (let pad = 0xec; codewords.length < DATA_CODEWORDS; pad ^= 0xfd) {
    codewords.push(pad);
  }

  return codewords;
}

function buildGaloisTables() {
  const exp = new Array<number>(512).fill(0);
  const log = new Array<number>(256).fill(0);
  let value = 1;

  for (let i = 0; i < 255; i += 1) {
    exp[i] = value;
    log[value] = i;
    value <<= 1;
    if (value & 0x100) {
      value ^= PRIMITIVE_POLYNOMIAL;
    }
  }

  for (let i = 255; i < exp.length; i += 1) {
    exp[i] = exp[i - 255];
  }

  return { exp, log };
}

const galois = buildGaloisTables();

function gfMultiply(left: number, right: number): number {
  if (left === 0 || right === 0) {
    return 0;
  }

  return galois.exp[galois.log[left] + galois.log[right]];
}

function buildGeneratorPolynomial(degree: number): number[] {
  let coefficients = [1];

  for (let i = 0; i < degree; i += 1) {
    const next = new Array<number>(coefficients.length + 1).fill(0);
    coefficients.forEach((coefficient, index) => {
      next[index] ^= coefficient;
      next[index + 1] ^= gfMultiply(coefficient, galois.exp[i]);
    });
    coefficients = next;
  }

  return coefficients;
}

function buildErrorCorrectionCodewords(data: number[]): number[] {
  const generator = buildGeneratorPolynomial(ECC_CODEWORDS);
  const remainder = new Array<number>(ECC_CODEWORDS).fill(0);

  data.forEach((codeword) => {
    const factor = codeword ^ remainder.shift()!;
    remainder.push(0);
    for (let i = 0; i < ECC_CODEWORDS; i += 1) {
      remainder[i] ^= gfMultiply(generator[i + 1], factor);
    }
  });

  return remainder;
}

function createEmptyMatrix() {
  return {
    modules: Array.from({ length: QR_SIZE }, () => Array<boolean>(QR_SIZE).fill(false)),
    reserved: Array.from({ length: QR_SIZE }, () => Array<boolean>(QR_SIZE).fill(false)),
  };
}

function isInBounds(x: number, y: number) {
  return x >= 0 && y >= 0 && x < QR_SIZE && y < QR_SIZE;
}

function setFunctionModule(
  modules: boolean[][],
  reserved: boolean[][],
  x: number,
  y: number,
  dark: boolean,
) {
  if (!isInBounds(x, y)) {
    return;
  }

  modules[y][x] = dark;
  reserved[y][x] = true;
}

function drawFinderPattern(modules: boolean[][], reserved: boolean[][], left: number, top: number) {
  for (let y = -1; y <= 7; y += 1) {
    for (let x = -1; x <= 7; x += 1) {
      const absoluteX = left + x;
      const absoluteY = top + y;
      if (!isInBounds(absoluteX, absoluteY)) {
        continue;
      }

      const isFinderArea = x >= 0 && x <= 6 && y >= 0 && y <= 6;
      const isDark = isFinderArea
        && (
          x === 0
          || x === 6
          || y === 0
          || y === 6
          || (x >= 2 && x <= 4 && y >= 2 && y <= 4)
        );
      setFunctionModule(modules, reserved, absoluteX, absoluteY, isDark);
    }
  }
}

function drawAlignmentPattern(modules: boolean[][], reserved: boolean[][], centerX: number, centerY: number) {
  for (let y = -2; y <= 2; y += 1) {
    for (let x = -2; x <= 2; x += 1) {
      setFunctionModule(
        modules,
        reserved,
        centerX + x,
        centerY + y,
        Math.max(Math.abs(x), Math.abs(y)) === 2 || (x === 0 && y === 0),
      );
    }
  }
}

function drawFunctionPatterns(modules: boolean[][], reserved: boolean[][]) {
  drawFinderPattern(modules, reserved, 0, 0);
  drawFinderPattern(modules, reserved, QR_SIZE - 7, 0);
  drawFinderPattern(modules, reserved, 0, QR_SIZE - 7);
  drawAlignmentPattern(modules, reserved, 30, 30);

  for (let i = 8; i < QR_SIZE - 8; i += 1) {
    const dark = i % 2 === 0;
    setFunctionModule(modules, reserved, i, 6, dark);
    setFunctionModule(modules, reserved, 6, i, dark);
  }

  setFunctionModule(modules, reserved, 8, QR_SIZE - 8, true);
  drawFormatBits(modules, reserved, 0);
}

function getFormatBits(maskPattern: number): number {
  const errorCorrectionLevel = 1;
  const data = (errorCorrectionLevel << 3) | maskPattern;
  let remainder = data;

  for (let i = 0; i < 10; i += 1) {
    remainder = (remainder << 1) ^ (((remainder >>> 9) & 1) * FORMAT_POLYNOMIAL);
  }

  return ((data << 10) | remainder) ^ FORMAT_MASK;
}

function getBit(value: number, index: number): boolean {
  return ((value >>> index) & 1) !== 0;
}

function drawFormatBits(modules: boolean[][], reserved: boolean[][], maskPattern: number) {
  const bits = getFormatBits(maskPattern);

  for (let i = 0; i <= 5; i += 1) {
    setFunctionModule(modules, reserved, 8, i, getBit(bits, i));
  }
  setFunctionModule(modules, reserved, 8, 7, getBit(bits, 6));
  setFunctionModule(modules, reserved, 8, 8, getBit(bits, 7));
  setFunctionModule(modules, reserved, 7, 8, getBit(bits, 8));
  for (let i = 9; i < 15; i += 1) {
    setFunctionModule(modules, reserved, 14 - i, 8, getBit(bits, i));
  }

  for (let i = 0; i < 8; i += 1) {
    setFunctionModule(modules, reserved, QR_SIZE - 1 - i, 8, getBit(bits, i));
  }
  for (let i = 8; i < 15; i += 1) {
    setFunctionModule(modules, reserved, 8, QR_SIZE - 15 + i, getBit(bits, i));
  }
  setFunctionModule(modules, reserved, 8, QR_SIZE - 8, true);
}

function maskBit(x: number, y: number): boolean {
  return (x + y) % 2 === 0;
}

function drawCodewords(modules: boolean[][], reserved: boolean[][], codewords: number[]) {
  const bits = codewords.flatMap((codeword) => (
    Array.from({ length: 8 }, (_, index) => (codeword >>> (7 - index)) & 1)
  ));
  let bitIndex = 0;
  let upward = true;

  for (let right = QR_SIZE - 1; right >= 1; right -= 2) {
    if (right === 6) {
      right -= 1;
    }

    for (let vertical = 0; vertical < QR_SIZE; vertical += 1) {
      const y = upward ? QR_SIZE - 1 - vertical : vertical;
      for (let offset = 0; offset < 2; offset += 1) {
        const x = right - offset;
        if (reserved[y][x]) {
          continue;
        }

        const rawBit = bitIndex < bits.length ? bits[bitIndex] === 1 : false;
        modules[y][x] = rawBit !== maskBit(x, y);
        bitIndex += 1;
      }
    }

    upward = !upward;
  }
}

export function createQrCodeMatrix(input: string): QrCodeMatrix {
  const data = buildDataCodewords(input);
  const errorCorrection = buildErrorCorrectionCodewords(data);
  const { modules, reserved } = createEmptyMatrix();

  drawFunctionPatterns(modules, reserved);
  drawCodewords(modules, reserved, [...data, ...errorCorrection]);
  drawFormatBits(modules, reserved, 0);

  return {
    size: QR_SIZE,
    modules,
  };
}

export function qrCodeToSvgPath(matrix: QrCodeMatrix, quietZone = 4): string {
  const commands: string[] = [];

  matrix.modules.forEach((row, y) => {
    row.forEach((isDark, x) => {
      if (isDark) {
        commands.push(`M${x + quietZone} ${y + quietZone}h1v1h-1z`);
      }
    });
  });

  return commands.join('');
}
