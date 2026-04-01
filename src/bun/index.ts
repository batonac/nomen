import { BrowserWindow, Updater } from "electrobun/bun";

const DEV_SERVER_PORT = 5173;
const DEV_SERVER_URL = `http://localhost:${DEV_SERVER_PORT}`;

// Check if Vite dev server is running for HMR
async function getMainViewUrl(): Promise<string> {
	const channel = await Updater.localInfo.channel();
	if (channel === "dev") {
		try {
			await fetch(DEV_SERVER_URL, { method: "HEAD" });
			console.log(`HMR enabled: Using Vite dev server at ${DEV_SERVER_URL}`);
			return DEV_SERVER_URL;
		} catch {
			console.log(
				"Vite dev server not running. Run 'bun run dev:hmr' for HMR support.",
			);
		}
	}
	return "views://mainview/index.html";
}

// Create the main application window
const url = await getMainViewUrl();

const mainWindow = new BrowserWindow({
	title: "Nomen",
	url,
	frame: {
		width: 1280,
		height: 860,
		x: 200,
		y: 200,
	},
});

console.log("Nomen main window created");

queueMicrotask(async () => {
	try {
		const { initializeDatabase } = await import("./db");
		const { path: databasePath } = initializeDatabase();
		console.log(`Nomen started with index at ${databasePath}`);
	} catch (error) {
		console.error("Failed to initialize database", error);
	}
});

void mainWindow;
