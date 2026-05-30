// Shared, pure draw primitives for the MatrixRain screensaver. Extracted so
// the fullscreen screensaver (MatrixRain.svelte) and the small config-panel
// preview (MatrixRainPreview.svelte) render an identical static frame without
// forking the constants or the per-cell color roll.
//
// High-fidelity Svelte adaptation of the MIT-licensed
// dcragusa/MatrixScreensaver project:
// https://github.com/dcragusa/MatrixScreensaver
// License notice: /static/matrix/LICENSE-MatrixScreensaver.txt

export const MATRIX_ALPHABET =
  "abcdefghijklmnopqrstuvwxyz123456789890~!#$%^&*()-_=+[]{};:'\",.<>/?\\|".split(
    "",
  );

export const COLUMN_SPACING_PX = 11;
export const ROW_SPACING_PX = 19;
export const RAIN_FONT_SIZE_PX = 20;

export const HEAD_COLOR = "#f6f6f4";
export const LEAD_COLOR = "#c9cfb9";
export const MID_COLOR = "#95a297";
export const BODY_COLOR = "#2cb231";

export function randInt(max: number): number {
  return Math.floor(Math.random() * max);
}

export function randomChar(): string {
  return MATRIX_ALPHABET[randInt(MATRIX_ALPHABET.length)] ?? "0";
}

// Grid dimensions derived from a canvas size. Both the screensaver and the
// preview size their canvas first, then ask for the column/row counts so the
// glyph grid lines up with the same spacing constants.
export function gridDimensions(width: number, height: number): {
  numCols: number;
  numChars: number;
} {
  return {
    numCols: Math.floor(width / COLUMN_SPACING_PX) + 1,
    numChars: Math.floor(height / ROW_SPACING_PX) + 1,
  };
}

// One static frame: a full grid of random glyphs, each colored by the same
// probability roll the live rain uses for its head/lead/mid/body bands. The
// caller owns the canvas; this only touches the passed 2d context. We clear
// first so a re-render does not stack on a prior frame.
export function drawStaticMatrix(
  ctx: CanvasRenderingContext2D,
  numCols: number,
  numChars: number,
): void {
  ctx.clearRect(
    0,
    0,
    numCols * COLUMN_SPACING_PX,
    numChars * ROW_SPACING_PX,
  );
  ctx.font = `${RAIN_FONT_SIZE_PX}px matrix_code`;
  for (let colIndex = 0; colIndex < numCols; colIndex += 1) {
    for (let rowIndex = 0; rowIndex < numChars; rowIndex += 1) {
      const roll = Math.random();
      if (roll < 0.04) {
        ctx.fillStyle = HEAD_COLOR;
      } else if (roll < 0.08) {
        ctx.fillStyle = LEAD_COLOR;
      } else if (roll < 0.12) {
        ctx.fillStyle = MID_COLOR;
      } else {
        ctx.fillStyle = BODY_COLOR;
      }
      ctx.fillText(
        randomChar(),
        colIndex * COLUMN_SPACING_PX,
        rowIndex * ROW_SPACING_PX + ROW_SPACING_PX,
      );
    }
  }
}
