# anonymous-discourse-post: *verifiable* anonymous post in Discourse (login not required)

Allow users to reply to a topic anonymously or create a new topic anonymously in Discourse, no login is required. This is the **backend** written in Rust using the Actix framework. For it to work, you also need to install the accompanying theme component: [discourse-anonymous-post](https://github.com/xjtumen/discourse-anonymous-post).

## Features
* **verifiable** anonymous post: users can capture XHR in their browser and find out that no extra information is passed to the server
* No login required: you can share a link with friends, and they can reply without signup. Useful for question-and-answer use-cases.
* Rate limited: to prevent spam, *reply* in rate limited to 3 times per 10 minutes and *new topic* is 1 times per 10 minutes by default. You can configure that in `main.rs`.
 
## How to set it up on *your* Discourse instance
We hard-code lots of assets' URLs since global CDN is slow for China, so you may have to manually serve files in `need-serve-elsewhere`.


```sh
cd ~
git clone https://github.com/xjtumen/anonymous-discourse-post
cd replytotopic
cargo build release
# create a new user called `instant_reply_agent`
# create an API key for it with sufficient permissions
# adapt replytotopic.service to your website, esp. DISCOURSE_API_KEY_ANONYMOUS, then:
cp replytotopic.service /etc/systemd/system/
systemctl enable --now replytotopic
# you need to configure a reverse proxy to http://127.0.0.1:7010
```


## TODO
* do not hard-encode rate limits
* add more friendly configuration method for easier setup
