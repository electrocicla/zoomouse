import { useState } from "react";
import type { ReactNode, PointerEvent } from "react";
import {
  Activity, ArrowLeft, BookOpen, Check, ChevronRight, CircleHelp, Crosshair,
  Eye, Globe, Keyboard, Laptop, LockKeyhole, Maximize2, Monitor, MonitorPlay,
  MousePointer2, Plus, Router, ScanLine, Settings2, ShieldCheck, SlidersHorizontal,
  Smartphone, Video, Wifi, X, ZoomIn
} from "lucide-react";
import type { ZoomSettings } from "@zoomouse/protocol";

type Screen = "connect" | "remote" | "settings" | "help";
type Host = { id: string; name: string; ip: string; label: string };
type SwitchProps = { checked: boolean; onChange: (next: boolean) => void; label: string };
type SectionProps = { title: string; children: ReactNode };
type MenuRowProps = { icon?: ReactNode; title: string; detail?: string; onClick?: () => void; trailing?: ReactNode };

const hosts: Host[] = [
  { id: "studio", name: "ZOOMOUSE-STUDIO", ip: "192.168.1.12", label: "PC principal · ejemplo" },
  { id: "office", name: "OFFICE-PC", ip: "192.168.1.28", label: "PC secundario · ejemplo" }
];

const defaults: ZoomSettings = {
  enabled: true, followCursor: true, magnification: 2.2, smoothing: 0.65, maxFps: 30
};

function Switch({ checked, onChange, label }: SwitchProps) {
  return <button type="button" role="switch" aria-checked={checked} aria-label={label}
    className={"switch" + (checked ? " switch-on" : "")} onClick={() => onChange(!checked)}>
    <span />
  </button>;
}

function Section({ title, children }: SectionProps) {
  return <section className="menu-section"><h3>{title}</h3><div className="section-content">{children}</div></section>;
}

function MenuRow({ icon, title, detail, onClick, trailing }: MenuRowProps) {
  return <div className="menu-row">
    <button type="button" className="menu-main" onClick={onClick} disabled={!onClick}>
      {icon && <span className="menu-icon">{icon}</span>}
      <span className="menu-copy"><strong>{title}</strong>{detail && <small>{detail}</small>}</span>
    </button>
    <span className="menu-trailing">{trailing ?? <ChevronRight size={18} className="row-chevron" />}</span>
  </div>;
}

function DemoDesktop({ zoom, cursor, move }: {
  zoom: ZoomSettings;
  cursor: { x: number; y: number };
  move: (x: number, y: number) => void;
}) {
  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (event.buttons === 0 && event.pointerType !== "touch") return;
    const rect = event.currentTarget.getBoundingClientRect();
    move(Math.round(Math.max(10, Math.min(90, (event.clientX - rect.left) / rect.width * 100))),
      Math.round(Math.max(10, Math.min(90, (event.clientY - rect.top) / rect.height * 100))));
  };
  return <div className="video-sim" onPointerDown={onPointerMove} onPointerMove={onPointerMove}
    role="img" aria-label="Simulación de una pantalla Windows con una región de zoom centrada en el cursor">
    <div className="scene" style={{
      transform: "scale(" + (zoom.enabled ? zoom.magnification : 1) + ")",
      transformOrigin: (zoom.followCursor ? cursor.x : 50) + "% " + (zoom.followCursor ? cursor.y : 50) + "%"
    }}>
      <div className="scene-top"><span>◉</span><span>ZOOMOUSE / VISTA DE EJEMPLO</span><span>— □ ×</span></div>
      <div className="scene-content">
        <div className="scene-sidebar"><span>⌂</span><span>▤</span><span>▦</span><span>◫</span></div>
        <div className="scene-cards">
          <div className="scene-kicker">WORKSPACE / DESIGN</div>
          <div className="scene-heading">Panel de trabajo</div>
          <div className="scene-card-grid">
            <div className="scene-card scene-blue"><span className="scene-card-symbol">◈</span><small>PROYECTOS</small><b>12 activos</b></div>
            <div className="scene-card scene-violet"><span className="scene-card-symbol">✳</span><small>TAREAS</small><b>8 en curso</b></div>
          </div>
          <div className="scene-chart"><span /><span /><span /><span /><span /><span /><span /><span /></div>
        </div>
      </div>
      <MousePointer2 className="scene-pointer" size={20} fill="white"
        style={{ left: cursor.x + "%", top: cursor.y + "%" }} />
    </div>
    <span className="video-watermark">DEMO · VIDEO SIMULADO</span>
    <span className="video-crosshair"><Crosshair size={14}/> Cursor follow</span>
  </div>;
}

function App() {
  const [screen, setScreen] = useState<Screen>("connect");
  const [activeHost, setActiveHost] = useState<Host>(hosts[0]!);
  const [zoom, setZoom] = useState<ZoomSettings>(defaults);
  const [cursor, setCursor] = useState({ x: 58, y: 45 });
  const [keyboardText, setKeyboardText] = useState("");
  const [manualIpVisible, setManualIpVisible] = useState(false);
  const [manualIp, setManualIp] = useState("");
  const [touchSpeed, setTouchSpeed] = useState(55);
  const [scrollSpeed, setScrollSpeed] = useState(48);
  const [naturalScroll, setNaturalScroll] = useState(true);
  const [leftHand, setLeftHand] = useState(false);
  const [gyro, setGyro] = useState(false);
  const [autoReconnect, setAutoReconnect] = useState(false);
  const [clickSound, setClickSound] = useState(true);
  const [hostApproved, setHostApproved] = useState(false);
  const [captureAllowed, setCaptureAllowed] = useState(false);
  const [showKeys, setShowKeys] = useState(false);
  const [helpItem, setHelpItem] = useState("Primeros pasos");

  const updateZoom = (patch: Partial<ZoomSettings>) => setZoom(current => ({ ...current, ...patch }));
  const openHost = (host: Host) => { setActiveHost(host); setScreen("remote"); };
  const menuIcon = (s: Screen) => s === "connect" ? <Wifi size={19} /> :
    s === "remote" ? <MousePointer2 size={19} /> : s === "settings" ? <Settings2 size={19} /> : <CircleHelp size={19} />;

  return <div className="app-root">
    <div className="topbar">
      <div className="brand"><span className="brand-mark"><MousePointer2 size={21} strokeWidth={2.5}/></span><span>zoo<span className="brand-accent">mouse</span></span></div>
      <div className="top-right"><span className="app-pill"><span className="pulse-dot"/> LAN FIRST</span><span className="app-version">PREVIEW / v0.1</span></div>
    </div>

    <div className="intro">
      <div><span className="eyebrow">CONTROL REMOTO, REIMAGINADO</span><h1>Tu PC, al alcance<br/><span>de tus dedos.</span></h1>
        <p>Un touchpad de precisión y una ventana inteligente que sigue tu cursor. Diseñado para Android + Windows 11.</p></div>
      <div className="intro-note"><ShieldCheck size={19}/><span>Prototipo visual. No hay dispositivos conectados, ni captura o inyección de entrada real.</span></div>
    </div>

    <main className="showcase">
      <section className="phone" aria-label="Vista previa de la app Android">
        <div className="phone-status"><span>9:41</span><div><Wifi size={15}/><span>▮▮▮</span><span className="battery">100</span></div></div>
        <div className="phone-header">
          {screen === "connect" ? <span className="page-icon"><Wifi size={19}/></span> :
            <button aria-label="Volver a dispositivos" className="back-btn" onClick={() => setScreen("connect")}><ArrowLeft size={20}/></button>}
          <span>{screen === "connect" ? "Conectar" : screen === "remote" ? activeHost.name : screen === "settings" ? "Ajustes" : "Ayuda"}</span>
          {screen === "connect" ? <button className="icon-button lime" aria-label="Agregar equipo" onClick={() => setManualIpVisible(!manualIpVisible)}><Plus size={24}/></button> : <span className="header-spacer" />}
        </div>

        <div className="phone-content" key={screen}>
          {screen === "connect" && <>
            <div className="phone-highlight"><span className="micro-label">RED LOCAL</span><div className="network-head"><span className="network-icon"><Router size={24}/></span><div><strong>Dispositivos disponibles</strong><small>Descubrimiento mDNS · próximamente</small></div></div></div>
            {manualIpVisible && <div className="manual-card"><label htmlFor="manual-ip">Conexión directa por IP (demo)</label><input id="manual-ip" placeholder="192.168.1.100" value={manualIp} onChange={e => setManualIp(e.target.value)} /><p>El campo es demostrativo: aún no se realiza una conexión.</p><button onClick={() => setManualIpVisible(false)}>Cerrar</button></div>}
            <div className="list-caption"><span>COMPUTADORES / EJEMPLOS</span><span>02</span></div>
            {hosts.map(host => <button key={host.id} className="device-row" onClick={() => openHost(host)}>
              <span className="device-icon"><Monitor size={24}/></span><span className="device-copy"><strong>{host.name}</strong><small>{host.label}</small></span><ChevronRight size={17}/></button>)}
            <div className="connect-actions"><button onClick={() => setManualIpVisible(true)}><Globe size={17}/> Conectar por IP</button><button onClick={() => setManualIpVisible(true)}><ScanLine size={17}/> Escanear QR</button></div>
            <div className="inline-note"><LockKeyhole size={16}/><span>El equipo Windows siempre deberá autorizar el acceso antes de activar el control o video.</span></div>
          </>}

          {screen === "remote" && <>
            <div className="remote-meta"><span><span className="orange-dot"/> SESIÓN DE PRUEBA</span><span>1080p · MOCK</span></div>
            <DemoDesktop zoom={zoom} cursor={cursor} move={(x,y) => setCursor({x,y})} />
            <div className="remote-video-controls"><button aria-label="Video simulado" className="control-pill selected"><Video size={17}/> Vista en vivo <span>DEMO</span></button><button aria-label="Vista completa simulada" className="square-button"><Maximize2 size={17}/></button></div>
            <div className="control-card"><div className="control-title"><ZoomIn size={17}/> ZOOM INTELIGENTE <span>{zoom.magnification.toFixed(1)}×</span></div>
              <div className="toggle-row"><div><strong>Zoom</strong><small>Ampliar una región de pantalla</small></div><Switch label="Activar zoom" checked={zoom.enabled} onChange={v=>updateZoom({enabled:v})}/></div>
              <div className="toggle-row"><div><strong>Seguir cursor</strong><small>El recorte persigue el puntero</small></div><Switch label="Seguir cursor" checked={zoom.followCursor} onChange={v=>updateZoom({followCursor:v})}/></div>
              <input aria-label="Nivel de zoom" type="range" min={1} max={4} step={0.1} value={zoom.magnification} disabled={!zoom.enabled} onChange={e=>updateZoom({magnification:Number(e.target.value)})}/>
              <div className="range-ticks"><span>1.0×</span><span>4.0×</span></div>
            </div>
            <div className="input-mode-row"><button className={!showKeys ? "active" : ""} onClick={()=>setShowKeys(false)}><MousePointer2 size={16}/> Touchpad</button><button className={showKeys ? "active" : ""} onClick={()=>setShowKeys(true)}><Keyboard size={16}/> Teclado</button></div>
            {showKeys ? <div className="keyboard-demo"><label htmlFor="demo-text">Entrada de teclado (solo demostración)</label><input id="demo-text" value={keyboardText} onChange={e=>setKeyboardText(e.target.value)} placeholder="Escribe aquí..." /><small>Sin envío real al PC.</small></div> :
              <div className="touchpad-demo" onPointerDown={event=>{const rect=event.currentTarget.getBoundingClientRect();setCursor({x:Math.round((event.clientX-rect.left)/rect.width*70+15),y:Math.round((event.clientY-rect.top)/rect.height*60+20)});}}>
                <MousePointer2 size={22}/><span>Desliza y toca aquí</span><small>Touchpad de ejemplo · sin control real</small></div>}
            <div className="mouse-buttons"><button onClick={()=>setCursor({x:Math.max(10,cursor.x-3),y:cursor.y})}>Clic izquierdo</button><button onClick={()=>setCursor({x:Math.min(90,cursor.x+3),y:cursor.y})}>Clic derecho</button></div>
          </>}

          {screen === "settings" && <>
            <Section title="INTERFAZ">
              <MenuRow icon={<SlidersHorizontal size={19}/>} title="Orden del panel" detail="Personaliza tus controles" onClick={()=>setLeftHand(!leftHand)} trailing={<span className="value-label">{leftHand?"Zurdo":"Normal"}</span>}/>
              <MenuRow title="Botón Gyro Mouse" detail="Mover puntero inclinando el móvil" onClick={()=>setGyro(!gyro)} trailing={<Switch label="Gyro Mouse" checked={gyro} onChange={setGyro}/>}/>
              <MenuRow title="Modo zurdo" detail="Invertir diseño y controles" onClick={()=>setLeftHand(!leftHand)} trailing={<Switch label="Modo zurdo" checked={leftHand} onChange={setLeftHand}/>}/>
            </Section>
            <Section title="PANEL TÁCTIL">
              <div className="slider-setting"><div>Velocidad de seguimiento <strong>{touchSpeed}%</strong></div><input aria-label="Velocidad de seguimiento" type="range" min={10} max={100} value={touchSpeed} onChange={e=>setTouchSpeed(Number(e.target.value))}/></div>
              <div className="slider-setting"><div>Velocidad de desplazamiento <strong>{scrollSpeed}%</strong></div><input aria-label="Velocidad de desplazamiento" type="range" min={10} max={100} value={scrollSpeed} onChange={e=>setScrollSpeed(Number(e.target.value))}/></div>
              <MenuRow title="Desplazamiento natural" detail="Contenido sigue el dedo" onClick={()=>setNaturalScroll(!naturalScroll)} trailing={<Switch label="Desplazamiento natural" checked={naturalScroll} onChange={setNaturalScroll}/>}/>
            </Section>
            <Section title="GENERAL">
              <MenuRow title="Reconexión automática" detail="Regresar al último host (opt-in)" onClick={()=>setAutoReconnect(!autoReconnect)} trailing={<Switch label="Reconexión automática" checked={autoReconnect} onChange={setAutoReconnect}/>}/>
              <MenuRow title="Sonido de clic" onClick={()=>setClickSound(!clickSound)} trailing={<Switch label="Sonido de clic" checked={clickSound} onChange={setClickSound}/>}/>
            </Section>
          </>}

          {screen === "help" && <>
            <div className="help-top"><BookOpen size={23}/><strong>Centro de ayuda</strong><span>Aprende a controlar Windows desde Android.</span></div>
            <Section title="PRIMEROS PASOS">
              {["Primera conexión","Gestos básicos","Zoom que sigue el cursor"].map(item=><div key={item}>
                <MenuRow title={item} onClick={()=>setHelpItem(helpItem===item?"":item)} trailing={<ChevronRight size={17}/>} />
                {helpItem===item && <p className="help-expanded">{item==="Primera conexión"?"Instala Zoomouse en Windows, abre la app Android, descubre el host y aprueba el emparejamiento local.":item==="Gestos básicos"?"Un dedo para mover; toca para clic, dos dedos para desplazamiento, y usa el panel Teclado para escribir.":"Activa Zoom y Seguir cursor; ajusta la escala y la ventana se centrará dinámicamente en el puntero."}</p>}
              </div>)}
            </Section>
            <Section title="OTROS CONTROLES">
              {["Multimedia y volumen","Navegador y aplicaciones","Atajos de teclado","Seguridad y privacidad","Preguntas frecuentes"].map(item=><MenuRow key={item} title={item} onClick={()=>setHelpItem(item)} trailing={helpItem===item?<Check size={17} className="lime"/>:<ChevronRight size={17}/>}/>)}
            </Section>
          </>}
        </div>

        <nav className="phone-nav" aria-label="Navegación del prototipo">
          {(["connect","remote","settings","help"] as const).map(s => <button key={s} className={screen===s?"current":""} onClick={()=>setScreen(s)} aria-label={s==="connect"?"Equipos":s==="remote"?"Control remoto":s==="settings"?"Ajustes":"Ayuda"}>{menuIcon(s)}<span>{s==="connect"?"Equipos":s==="remote"?"Control":s==="settings"?"Ajustes":"Ayuda"}</span></button>)}
        </nav>
        <div className="phone-home"><span/></div>
      </section>

      <aside className="control-side">
        <div className="side-label"><span className="micro-label">WINDOWS 11 / HOST</span><span className="demo-tag">SIMULACIÓN</span></div>
        <div className="host-dashboard">
          <div className="host-head"><div className="host-badge"><MonitorPlay size={24}/></div><div><strong>Zoomouse Host</strong><span>Servicio de sesión · concepto</span></div><span className="pending">DEMO</span></div>
          <div className="host-machine"><Laptop size={20}/><span>ZOOMOUSE-STUDIO</span><span className="host-os">Win 11</span></div>
          <div className="host-status"><span className="status-dot"/><span>Esperando permiso local</span></div>
          <div className="approval-box"><div className="approval-top"><ShieldCheck size={20}/><div><strong>Solicitud de emparejamiento</strong><small>Android · dispositivo de ejemplo</small></div></div>
            <div className="host-actions">{hostApproved ? <button className="approved" onClick={()=>{setHostApproved(false);setCaptureAllowed(false);}}><Check size={16}/> Permiso demo concedido · revocar</button> : <button className="approve-btn" onClick={()=>setHostApproved(true)}><Check size={16}/> Autorizar ejemplo</button>}
              <button className="decline-btn" onClick={()=>{setHostApproved(false);setCaptureAllowed(false);}}><X size={16}/> Denegar</button></div>
          </div>
          <div className="host-privacy"><div><Eye size={17}/><span>Permitir captura de pantalla</span></div><Switch label="Captura simulada" checked={captureAllowed} onChange={v=>setCaptureAllowed(hostApproved&&v)}/></div>
          <div className="host-footer"><LockKeyhole size={15}/> El vídeo y el control real estarán bloqueados hasta autorización verificable.</div>
        </div>
        <div className="side-heading">Una sola experiencia.<br/><span>Dos dispositivos.</span></div>
        <div className="feature-list">
          <div><span className="feature-icon blue"><Wifi size={20}/></span><div><strong>Conexión LAN instantánea</strong><p>Detección local, QR y entrada por IP. Internet en la siguiente fase.</p></div></div>
          <div><span className="feature-icon lime-bg"><Crosshair size={20}/></span><div><strong>Cursor Follow Zoom</strong><p>Recorte GPU que sigue al puntero y muestra únicamente lo relevante.</p></div></div>
          <div><span className="feature-icon purple"><Activity size={20}/></span><div><strong>Baja latencia</strong><p>Rust, captura nativa y WebRTC con códecs por hardware.</p></div></div>
        </div>
        <div className="architecture-strip"><span><Smartphone size={17}/> React Native</span><ChevronRight size={16}/><span><Router size={17}/> WebRTC / LAN</span><ChevronRight size={16}/><span><Monitor size={17}/> Rust + Tauri</span></div>
        <div className="side-warning"><ShieldCheck size={17}/><p>Todos los controles de esta pantalla son simulados y no envían eventos a Windows. La fase LAN requiere desarrollo nativo y pruebas E2E.</p></div>
      </aside>
    </main>
    <footer className="footer">ZOOMOUSE <span>·</span> LAN FIRST <span>·</span> OPEN SOURCE <span>·</span> VISTA PREVIA INTERACTIVA</footer>
  </div>;
}

export default App;
