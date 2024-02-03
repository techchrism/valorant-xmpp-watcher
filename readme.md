# Valorant XMPP Watcher

A tool to connect to Riot XMPP servers to watch for presence updates and chat messages.

Exports XMPP logs in a format compatible with [Valorant XMPP Log Viewer](https://github.com/techchrism/valorant-xmpp-log-viewer)

## Usage
- Clone the repo and run `npm install`
- Run `npm run build` to build the project
- Obtain the account cookies and put them in `cookies.txt`
  - You can use [Riot Cookie String Generator](https://riotcookiestringgen.techchrism.me/) to quickly obtain your cookies
  - Alternatively, you can put the cookies in the environment variable `XMPP_WATCHER_STARTING_COOKIES` which the xmpp watcher will use if `cookies.txt` is not found
- Run `node .` to start the watcher
