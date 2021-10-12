<img src="https://raw.githubusercontent.com/frostu8/kromer/main/kromer.png" alt="kromer.png"/>

A cool Discord bot for cool servers.

Discord is an open, free platform. So why should we continue paying to make our
Discord servers the best they can be? A good bot should be open and free, just
like the platform, and that's what this aims to be.

If you're not a developer and care only for the bot itself, you can
[invite it here](https://discord.com/api/oauth2/authorize?client_id=895420881696849920&permissions=0&scope=bot%20applications.commands).
But if you **are** a user, keep in mind that this bot is not only unfinished,
but this idea is so unbelieveably unoriginal that there are
[a whole bunch of better alternatives](https://github.com/jacc/awesome-discord).
My personal favorite is [blargbot](https://blargbot.xyz/), but that's an
opinion from a Rust programmer, so take it with a grain of salt and do what you
need to do to make your server a good one.

Features:

* **Server levelling system**  
  Send messages to increase your Kromer balance, and compare yourself with
  others to see who's the most active member.
* **Reaction roles**  
  Allow members to self-assign themselves some roles through a reaction on a
  message.
* **Up-to-date with the latest Discord trends**  
  The Discord developers were nice enough to give bots a whole bunch of tools
  that make bots feel like an *integration* rather than a *hack*, and this bot
  intends on using all of those features. Users shouldn't have to enable
  Developer Mode to setup reaction roles.
* **Powered by Rust and built with Twilight**  
  Rust is an insanely fast systems language, and Twilight is a no-compensations
  Discord library. Nothing stopping you from throwing this up on a Raspberry Pi
  and hosting possibly hundreds of servers... *probably*.

Are you tired of all the DELTARUNE propaganda in the official bot? Why not
[host your own?](#hosting-kromer)

# Hosting `kromer`
Not only can you host kromer, but you are encouraged to! Feel free to poke
around at the internals while you're at it, but by no means it is required.
What *is* required is some basic shell knowledge, a
[**Discord application** set up with a **bot acount**][1], [**PostgreSQL**][2]
and some of your time and love 🥰.

Once you got everything fired up, you'll need to set these environment 
variables:

* `DATABASE_URL`: The connection uri used by the bot to connect to the
  database. See [this page][3] if you don't know how to write a connection uri,
  or you're just stuck. Don't worry, it happens to the best of us.
* `DISCORD_TOKEN`: The token of your Discord bot. This should be under the
  "**Bot**" section of [your application][1]. If it wasn't apparent enough,
  this **should be kept** ***very*** **secret**.

Once you've set those environment variables, just run the bot and watch it go!
It will automatically set up the database and initialize the global commands.
It takes an hour at most to initialize the global commands, but once that's
complete, you'll be raring to go!

## License
This project is licensed under `The Unlicense`, which is just a fancy way of
saying "do absolutely anything you want with my code, no permission or annoying
license management necessary." Reference it, copy it, maim it, or even sell it;
as long as my code is making someone's day, I'm happy.

[1]: https://discord.com/developers/applications
[2]: https://www.postgresql.org/
[3]: https://www.postgresql.org/docs/9.3/libpq-connect.html#AEN39692

