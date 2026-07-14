import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { converter, formatHex, wcagContrast } from "culori";

const toOklch = converter("oklch");
const toRgb = converter("rgb");

// Color roles (Material Design 3 inspired)
// These are semantic color roles, NOT tied to backgrounds
export enum AccentRole {
	Primary = "Primary",
	Secondary = "Secondary",
	Tertiary = "Tertiary",
	Container = "Container",
	Fill = "Fill",
	Border = "Border",
	Icon = "Icon",
	Glow = "Glow",
}

export enum OnRole {
	OnPrimary = "OnPrimary",
	OnSecondary = "OnSecondary",
	OnTertiary = "OnTertiary",
	OnContainer = "OnContainer",
}

export enum Surface {
	Background = "Background",
	Secondary = "Secondary",
	Card = "Card",
	CardHover = "CardHover",
	Hover = "Hover",
	Input = "Input",
	Glass = "Glass",
	Sidebar = "Sidebar",
	Dialog = "Dialog",
	Surface = "Surface",
	Navbar = "Navbar",
	Panel = "Panel",
}

const SURFACE_TO_CSS: Record<Surface, string> = {
	[Surface.Background]: "--bg-primary",
	[Surface.Secondary]: "--bg-secondary",
	[Surface.Card]: "--bg-card",
	[Surface.CardHover]: "--bg-card-hover",
	[Surface.Hover]: "--bg-hover",
	[Surface.Input]: "--bg-input",
	[Surface.Glass]: "--bg-glass",
	[Surface.Sidebar]: "--bg-sidebar",
	[Surface.Dialog]: "--bg-dialog",
	[Surface.Surface]: "--bg-surface",
	[Surface.Navbar]: "--bg-navbar",
	[Surface.Panel]: "--bg-panel",
} as const;

const ALL_SURFACES = Object.values(Surface);

const THEME_PRESET_MAP = {
	darkToLight: {
		"#a778eb": "#53158f",
		"#d9af09": "#d49f02",
	},
	lightToDark: {
		"#53158f": "#a778eb",
		"#d49f02": "#d9af09",
	},
} as const;

interface OklchColor {
	mode: "oklch";
	l: number;
	c: number;
	h: number;
}

const MIN_CHROMA = 0.08;
const MAX_CHROMA = 0.18;

function getOklch(color: string): OklchColor {
	const c = toOklch(color);
	// Achromatic colors (white, black, grays) have no defined hue in culori —
	// that's expected, not an error. Default h to 0 instead of bailing out,
	// otherwise every white/black accent silently skips contrast correction.
	return {
		mode: "oklch",
		l: c?.l ?? 0,
		c: c?.c ?? 0,
		h: c?.h ?? 0,
	};
}

function fromOklch(l: number, c: number, h: number): string {
	return formatHex({
		mode: "oklch",
		l: Math.max(0, Math.min(1, l)),
		c: Math.max(0, Math.min(MAX_CHROMA * 2, c)),
		h: h % 360,
	});
}

/**
 * Returns a version of `color` guaranteed to meet `minRatio` contrast against
 * `background`, preserving hue/chroma as much as possible.
 *
 * Strategy: WCAG contrast moves monotonically as lightness moves away from
 * the background's lightness, so a single-direction binary search on L is
 * enough — no need to probe both directions or escalate chroma in stages.
 * If no lightness at this hue/chroma can reach the target (rare, only near
 * mid-gray backgrounds), fall back to pure white/black, which is guaranteed
 * to hit WCAG AA against virtually any background.
 */
function getAccessibleColor(
	color: string,
	background: string,
	minRatio: number = 4.5
): string {
	if (wcagContrast(color, background) >= minRatio) {
		return color;
	}

	const { l, c, h } = getOklch(color);
	const bgIsDark = getOklch(background).l < 0.5;

	let low = bgIsDark ? l : 0;
	let high = bgIsDark ? 1 : l;
	let best = color;
	let bestRatio = wcagContrast(color, background);

	for (let i = 0; i < 24; i++) {
		const mid = (low + high) / 2;
		const candidate = fromOklch(mid, c, h);
		const ratio = wcagContrast(candidate, background);

		if (ratio > bestRatio) {
			best = candidate;
			bestRatio = ratio;
		}

		const closeEnough = ratio >= minRatio;
		if (bgIsDark) {
			if (closeEnough) high = mid;
			else low = mid;
		} else {
			if (closeEnough) low = mid;
			else high = mid;
		}
	}

	return bestRatio >= minRatio ? best : getOnColor(background);
}

function getOnColor(background: string): "#ffffff" | "#000000" {
	const whiteContrast = wcagContrast("#ffffff", background);
	const blackContrast = wcagContrast("#000000", background);
	return whiteContrast > blackContrast ? "#ffffff" : "#000000";
}

function getRgbComponents(hex: string): { r: number; g: number; b: number } {
	const color = toRgb(hex);
	if (!color) return { r: 0, g: 0, b: 0 };
	return {
		r: Math.round((color.r ?? 0) * 255),
		g: Math.round((color.g ?? 0) * 255),
		b: Math.round((color.b ?? 0) * 255),
	};
}

// Scales chroma into [MIN_CHROMA, cap] like before, but leaves truly neutral
// input (white/black/gray, c ≈ 0) untouched instead of injecting an arbitrary
// tint — otherwise every role derived from a gray accent would pick up a
// reddish hue from the h=0 default in getOklch.
function scaledChroma(baseChroma: number, factor: number = 1, cap: number = MAX_CHROMA): number {
	if (baseChroma < 0.005) return 0;
	return Math.max(MIN_CHROMA, Math.min(cap, baseChroma * factor));
}

function generateAccentRoles(accent: string): Record<AccentRole, string> {
	const oklch = getOklch(accent);
	const primary = accent;
	const secondary = fromOklch(
		Math.max(0, oklch.l - 0.08),
		scaledChroma(oklch.c),
		oklch.h
	);
	const tertiary = fromOklch(
		Math.max(0, oklch.l - 0.15),
		scaledChroma(oklch.c, 0.8),
		oklch.h
	);
	const containerL = Math.max(0.3, Math.min(0.7, oklch.l));
	const container = fromOklch(containerL, scaledChroma(oklch.c), oklch.h);
	const fillL = containerL * 0.95;
	const fill = fromOklch(fillL, scaledChroma(oklch.c, 0.9), oklch.h);
	const borderL = oklch.l > 0.5 ? Math.max(0.4, oklch.l - 0.1) : Math.min(0.6, oklch.l + 0.1);
	const border = fromOklch(borderL, scaledChroma(oklch.c, 1.1), oklch.h);
	const iconL = borderL;
	const icon = fromOklch(iconL, scaledChroma(oklch.c, 1.1), oklch.h);
	const glow = fromOklch(oklch.l, scaledChroma(oklch.c, 1.3, MAX_CHROMA * 1.5), oklch.h);

	return {
		[AccentRole.Primary]: primary,
		[AccentRole.Secondary]: secondary,
		[AccentRole.Tertiary]: tertiary,
		[AccentRole.Container]: container,
		[AccentRole.Fill]: fill,
		[AccentRole.Border]: border,
		[AccentRole.Icon]: icon,
		[AccentRole.Glow]: glow,
	};
}

function generateOnRoles(accentRoles: Record<AccentRole, string>): Record<OnRole, string> {
	return {
		[OnRole.OnPrimary]: getOnColor(accentRoles[AccentRole.Primary]),
		[OnRole.OnSecondary]: getOnColor(accentRoles[AccentRole.Secondary]),
		[OnRole.OnTertiary]: getOnColor(accentRoles[AccentRole.Tertiary]),
		[OnRole.OnContainer]: getOnColor(accentRoles[AccentRole.Container]),
	};
}

export interface AppearanceSettings {
	theme: "system" | "dark" | "light";
	accent_color: string;
	custom_presets: unknown[];
}

export interface CustomPreset {
	hex: string;
	name: string;
}

export type AccentRoles = Readonly<Record<AccentRole, string>>;
export type OnRoles = Readonly<Record<OnRole, string>>;
export type AccentVariants = Readonly<Record<Surface, string>>;

const MAX_CUSTOM_PRESETS = 8;

// Los presets personalizados se guardaban antes como un array plano de hex
// strings (sin nombre). Para no romper instalaciones existentes al agregar
// nombres, aceptamos ambos formatos acá y los normalizamos a { hex, name }.
function normalizeCustomPresets(raw: unknown): CustomPreset[] {
	if (!Array.isArray(raw)) return [];
	return raw
		.map((item, i): CustomPreset | null => {
			if (typeof item === "string") {
				return { hex: item, name: `Custom ${i + 1}` };
			}
			if (item && typeof item === "object" && "hex" in item) {
				const obj = item as Partial<CustomPreset>;
				if (!obj.hex) return null;
				const name = obj.name?.trim();
				return { hex: obj.hex, name: name && name.length > 0 ? name : `Custom ${i + 1}` };
			}
			return null;
		})
		.filter((p): p is CustomPreset => p !== null);
}

export const useThemeStore = defineStore("theme", () => {
	const theme = ref<"system" | "dark" | "light">("system");
	const accentColor = ref("#000000");
	const customPresets = ref<CustomPreset[]>([]);
	const isInitialized = ref(false);
	const isDarkMode = ref(true);

	const accentRoles = ref<AccentRoles>({} as AccentRoles);
	const onRoles = ref<OnRoles>({} as OnRoles);
	const accentVariants = ref<AccentVariants>({} as AccentVariants);

	const defaultPresets = computed(() => {
		const dark = resolveTheme() === "dark";

		return [
			{
				id: "purple",
				hex: dark ? "#a778eb" : "#53158f",
				label: "Purple",
			},
			{
				id: "amber",
				hex: dark ? "#d9af09" : "#d49f02",
				label: "Yellow",
			},
		];
	});

	const allPresets = computed(() => [
		...defaultPresets.value,
		...customPresets.value.map((preset, i) => ({
			id: `custom-${i}-${preset.hex}`,
			hex: preset.hex,
			label: preset.name,
		})),
	]);

	function resolveTheme(): "dark" | "light" {
		if (theme.value === "system") {
			return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
		}
		return theme.value;
	}

	function generateRoles() {
		const roles = generateAccentRoles(accentColor.value);
		accentRoles.value = roles;
		onRoles.value = generateOnRoles(roles);
	}

	function precomputeAccentVariants() {
		const variants: Partial<Record<Surface, string>> = {};
		for (const surface of ALL_SURFACES) {
			const cssVar = SURFACE_TO_CSS[surface];
			const bg = getComputedStyle(document.documentElement).getPropertyValue(cssVar).trim();
			if (bg && bg !== "") {
				variants[surface] = getAccessibleColor(accentRoles.value[AccentRole.Fill], bg, 4.5);
			}
		}
		accentVariants.value = variants as AccentVariants;
	}

	function applyTheme() {
		const previousTheme = isDarkMode.value ? "dark" : "light";
		const resolved = resolveTheme();
		isDarkMode.value = resolved === "dark";

		// Si el usuario tiene seleccionado un preset por defecto,
		// conviértelo automáticamente al equivalente del otro tema.
		if (previousTheme !== resolved) {
			if (resolved === "light") {
				accentColor.value =
					THEME_PRESET_MAP.darkToLight[
						accentColor.value as keyof typeof THEME_PRESET_MAP.darkToLight
					] ?? accentColor.value;
			} else {
				accentColor.value =
					THEME_PRESET_MAP.lightToDark[
						accentColor.value as keyof typeof THEME_PRESET_MAP.lightToDark
					] ?? accentColor.value;
			}
		}

		const root = document.documentElement;
		root.classList.remove("dark", "light");
		root.classList.add(resolved);

		generateRoles();

		const userColor = accentColor.value;
		const rgb = getRgbComponents(userColor);
		const r = rgb.r, g = rgb.g, b = rgb.b;

		for (const [role, color] of Object.entries(accentRoles.value)) {
			root.style.setProperty(`--accent-${role.toLowerCase()}`, color);
		}

		for (const [role, color] of Object.entries(onRoles.value)) {
			const roleName = role.replace('On', '').toLowerCase();
			root.style.setProperty(`--on-accent-${roleName}`, color);
		}

		precomputeAccentVariants();
		
		for (const [surface, color] of Object.entries(accentVariants.value)) {
			root.style.setProperty(`--accent-${surface.toLowerCase()}`, color);
		}

		// Legacy support
		root.style.setProperty("--accent-primary", userColor);
		root.style.setProperty("--accent-display", userColor);
		root.style.setProperty("--accent-secondary", accentRoles.value[AccentRole.Secondary]);
		root.style.setProperty("--accent-tertiary", accentRoles.value[AccentRole.Tertiary]);

		if (resolved === "dark") {
			root.style.setProperty("--accent-glow", `rgba(${r}, ${g}, ${b}, 0.15)`);
			root.style.setProperty("--accent-glow-strong", `rgba(${r}, ${g}, ${b}, 0.35)`);
			root.style.setProperty("--border-hover", `rgba(${r}, ${g}, ${b}, 0.25)`);
			root.style.setProperty("--border-active", `rgba(${r}, ${g}, ${b}, 0.4)`);
			root.style.setProperty("--border-glow", `rgba(${r}, ${g}, ${b}, 0.08)`);
			root.style.setProperty("--titlebar-bg", `rgba(10, 10, 22, 0.82)`);
			root.style.setProperty("--titlebar-border", `rgba(${r}, ${g}, ${b}, 0.12)`);
			root.style.setProperty("--titlebar-text", "#f0f0f8");
			root.style.setProperty("--titlebar-text-muted", "#6b6b8a");
			root.style.setProperty("--titlebar-btn-hover", `rgba(${r}, ${g}, ${b}, 0.12)`);
			root.style.setProperty("--titlebar-glow-line", `rgba(${r}, ${g}, ${b}, 0.25)`);
		} else {
			root.style.setProperty("--accent-glow", `rgba(${r}, ${g}, ${b}, 0.12)`);
			root.style.setProperty("--accent-glow-strong", `rgba(${r}, ${g}, ${b}, 0.28)`);
			root.style.setProperty("--border-hover", `rgba(${r}, ${g}, ${b}, 0.22)`);
			root.style.setProperty("--border-active", `rgba(${r}, ${g}, ${b}, 0.35)`);
			root.style.setProperty("--border-glow", `rgba(${r}, ${g}, ${b}, 0.06)`);
			root.style.setProperty("--titlebar-bg", `rgba(255, 255, 255, 0.82)`);
			root.style.setProperty("--titlebar-border", `rgba(${r}, ${g}, ${b}, 0.15)`);
			root.style.setProperty("--titlebar-text", "#1a1a2e");
			root.style.setProperty("--titlebar-text-muted", "#6b6b80");
			root.style.setProperty("--titlebar-btn-hover", `rgba(${r}, ${g}, ${b}, 0.1)`);
			root.style.setProperty("--titlebar-glow-line", `rgba(${r}, ${g}, ${b}, 0.2)`);
		}
	}

	function setAccent(hex: string) {
		accentColor.value = hex;
		applyTheme();
	}

	function setTheme(newTheme: "system" | "dark" | "light") {
		theme.value = newTheme;
		applyTheme();
	}

	function addCustomPreset(hex: string, name?: string): boolean {
		const clean = hex.toLowerCase();
		if (customPresets.value.length >= MAX_CUSTOM_PRESETS) return false;
		if (customPresets.value.some((p) => p.hex.toLowerCase() === clean)) return false;
		const cleanName = name?.trim();
		customPresets.value.push({
			hex: clean,
			name: cleanName && cleanName.length > 0 ? cleanName : `Custom ${customPresets.value.length + 1}`,
		});
		return true;
	}

	function updateCustomPreset(index: number, patch: Partial<CustomPreset>): boolean {
		const preset = customPresets.value[index];
		if (!preset) return false;

		const nextHex = (patch.hex ?? preset.hex).toLowerCase();
		const nextName = patch.name?.trim();

		// Evitar quedar con dos presets del mismo color
		if (customPresets.value.some((p, i) => i !== index && p.hex.toLowerCase() === nextHex)) {
			return false;
		}

		customPresets.value.splice(index, 1, {
			hex: nextHex,
			name: nextName && nextName.length > 0 ? nextName : preset.name,
		});
		return true;
	}

	function removeCustomPreset(index: number) {
		if (index >= 0 && index < customPresets.value.length) {
			customPresets.value.splice(index, 1);
		}
	}

	function initFromSettings(settings: Partial<AppearanceSettings>) {
		if (isInitialized.value) return;
		theme.value = settings.theme ?? "system";
		accentColor.value = settings.accent_color ?? "#000000";
		customPresets.value = normalizeCustomPresets(settings.custom_presets);
		isInitialized.value = true;
		applyTheme();
	}

	function getAccentForSurface(surface: Surface): string {
		return accentVariants.value[surface];
	}

	function getAccentForRole(role: AccentRole): string {
		return accentRoles.value[role];
	}

	function getOnForRole(role: OnRole): string {
		return onRoles.value[role];
	}

	let mql: MediaQueryList | null = null;
	if (typeof window !== "undefined") {
		mql = window.matchMedia("(prefers-color-scheme: dark)");
		mql.addEventListener("change", () => {
			if (theme.value === "system") applyTheme();
		});
	}

	return {
		theme,
		accentColor,
		customPresets,
		defaultPresets,
		allPresets,
		isInitialized,
		isDarkMode,
		accentRoles,
		onRoles,
		accentVariants,
		getAccentForSurface,
		getAccentForRole,
		getOnForRole,
		setTheme,
		setAccent,
		addCustomPreset,
		updateCustomPreset,
		removeCustomPreset,
		applyTheme,
		initFromSettings,
		resolveTheme,
	};
});