declare module 'culori' {
  interface Color {
    mode: string;
    l?: number;
    c?: number;
    h?: number;
    r?: number;
    g?: number;
    b?: number;
    [key: string]: any;
  }

  interface Converter {
    (color: string): Color;
    (color: Color): Color;
  }

  export function converter(mode: string): Converter;
  export function formatHex(color: Color): string;
  export function wcagContrast(a: string, b: string): number;
}
