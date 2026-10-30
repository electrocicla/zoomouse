import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";

afterEach(() => cleanup());

const openDemoRemote = async () => {
  const user = userEvent.setup();
  render(<App />);
  await user.click(screen.getByRole("button", { name: /ZOOMOUSE-STUDIO/ }));
  return user;
};

describe("Zoomouse M0 interactive preview", () => {
  it("separates QR onboarding from direct IP, without claiming a live connection", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Escanear QR" }));
    expect(screen.getByText("Escanear código QR (demo)")).toBeDefined();
    expect(screen.queryByLabelText("Conexión directa por IP (demo)")).toBeNull();
    await user.click(screen.getByRole("button", { name: "Conectar por IP" }));
    expect(screen.getByLabelText("Conexión directa por IP (demo)")).toBeDefined();
    expect(screen.queryByText("Escanear código QR (demo)")).toBeNull();
  });

  it("freezes the simulated crop anchor when tracking is disabled, and resumes on reactivation", async () => {
    const user = await openDemoRemote();
    const video = screen.getByRole("group", { name: /Pantalla Windows simulada/ });
    const scene = video.querySelector(".scene");
    expect(scene?.getAttribute("style")).toContain("58% 45%");
    await user.click(screen.getByRole("switch", { name: "Seguir cursor" }));
    fireEvent.keyDown(video, { key: "ArrowRight" });
    expect(scene?.getAttribute("style")).toContain("58% 45%");
    await user.click(screen.getByRole("switch", { name: "Seguir cursor" }));
    expect(scene?.getAttribute("style")).toContain("61% 45%");
    fireEvent.keyDown(video, { key: "ArrowRight" });
    expect(scene?.getAttribute("style")).toContain("64% 45%");
  });

  it("toggles zoom independently from tracking and expands the simulated video", async () => {
    const user = await openDemoRemote();
    const video = screen.getByRole("group", { name: /Pantalla Windows simulada/ });
    const scene = video.querySelector(".scene");
    await user.click(screen.getByRole("switch", { name: "Activar zoom" }));
    expect(scene?.getAttribute("style")).toContain("scale(1)");
    expect(screen.getByRole("switch", { name: "Seguir cursor" }).getAttribute("aria-checked")).toBe("true");
    await user.click(screen.getByRole("button", { name: "Ampliar video simulado" }));
    expect(video.className).toContain("video-expanded");
    await user.click(screen.getByRole("button", { name: "Contraer video simulado" }));
    expect(video.className).not.toContain("video-expanded");
  });

  it("reports fake pointer clicks without moving the cursor or sending input", async () => {
    const user = await openDemoRemote();
    const video = screen.getByRole("group", { name: /Pantalla Windows simulada/ });
    const scene = video.querySelector(".scene");
    const before = scene?.getAttribute("style");
    await user.click(screen.getByRole("button", { name: "Clic derecho" }));
    expect(screen.getByText(/Clic derecho simulado/)).toBeDefined();
    expect(scene?.getAttribute("style")).toBe(before);
  });

  it("tracks drag gestures within the simulated touchpad", async () => {
    await openDemoRemote();
    const pad = screen.getByText("Desliza y toca aquí").closest(".touchpad-demo");
    if (!pad) throw new Error("touchpad preview missing");
    vi.spyOn(pad, "getBoundingClientRect").mockReturnValue({
      x: 0, y: 0, left: 0, top: 0, right: 100, bottom: 100, width: 100, height: 100,
      toJSON: () => ({})
    });
    const video = screen.getByRole("group", { name: /Pantalla Windows simulada/ });
    const scene = video.querySelector(".scene");
    fireEvent.pointerDown(pad, { pointerId: 1, clientX: 20, clientY: 40, buttons: 1 });
    expect(scene?.getAttribute("style")).toContain("29% 44%");
    fireEvent.pointerMove(pad, { pointerId: 1, clientX: 80, clientY: 60, buttons: 1 });
    expect(scene?.getAttribute("style")).toContain("71% 56%");
  });

  it("gates the capture toggle on Windows-side demo approval and resets on revoke", async () => {
    const user = userEvent.setup();
    render(<App />);
    const capture = screen.getByRole("switch", { name: "Captura simulada" });
    await user.click(capture);
    expect(capture.getAttribute("aria-checked")).toBe("false");
    await user.click(screen.getByRole("button", { name: /Autorizar ejemplo/ }));
    await user.click(capture);
    expect(capture.getAttribute("aria-checked")).toBe("true");
    expect(screen.getByText("Control y vídeo simulados autorizados")).toBeDefined();
    await user.click(screen.getByRole("button", { name: /Permiso demo concedido/ }));
    expect(capture.getAttribute("aria-checked")).toBe("false");
  });

  it("keeps the settings independent and exposes documentation for planned features", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Ajustes" }));
    await user.click(screen.getByRole("button", { name: /Orden del panel/ }));
    expect(screen.getByText("Invertido")).toBeDefined();
    expect(screen.getByRole("switch", { name: "Modo zurdo" }).getAttribute("aria-checked")).toBe("false");
    await user.click(screen.getByText("Modo zurdo"));
    expect(screen.getByRole("switch", { name: "Modo zurdo" }).getAttribute("aria-checked")).toBe("true");
    await user.click(screen.getByRole("button", { name: "Ayuda" }));
    await user.click(screen.getByRole("button", { name: /Navegador y aplicaciones/ }));
    expect(screen.getByText(/Funcionalidad prevista para M3/)).toBeDefined();
  });
});
