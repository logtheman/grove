import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/globals.css";

// StrictMode disabled: it double-mounts components, which spawns duplicate PTY sessions
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <App />,
);
