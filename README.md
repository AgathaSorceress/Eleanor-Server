# Eleanor Server 

A HTTP remote media server for the [Eleanor](https://github.com/AgathaSorceress/Eleanor) music player.

## Usage (command line)

- Setup:
	1. Run the `eleanor-server` executable; exit 
	2. Edit the `settings.toml` file. Example:
	```toml 
	port = 8008

	[[sources]]
	id = 0
	path = "/home/agatha/Music/local/"
	```
	3. Add user(s) using the `user` subcommand 
	4. Start the server again 

- Adding a user:
	```sh
	./eleanor-server user add username password
	```
- Removing a user:
	```sh 
	./eleanor-server user remove username 
	```

## Development
A testing server can be quickly set up in a temporary directory like this:
```
nix develop .#dev
```

## REST API Implementation 

`GET /`            → A binary messagepack-encoded list of tracks indexed by the server    
`POST /`           → Starts a full reindex by the server    
`GET /:hash `      → Responds with the requested song's audio file (hash can be obtained from the index)   
`GET /:hash/cover` → Responds with the album art attached to the requested song's file 

All endpoints require HTTP Basic Auth using credentials of any formerly added user.
