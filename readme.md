# Valorant XMPP Watcher

A tool to connect to Riot XMPP servers to watch for presence updates and chat messages.

Exports XMPP logs in a format compatible with [Valorant XMPP Log Viewer](https://github.com/techchrism/valorant-xmpp-log-viewer)

## Usage
- Clone the repo and run `npm install`
- Run `npm run build` to build the project
- If necessary, edit `config.json` to change the XMPP server / id
- Obtain an account SSID cookie and put it in `ssid.txt`
- Run `node .` to start the watcher
