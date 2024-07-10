# Documentation

This repo contains rust code to perform rsync push operations on file changes in a local directory to a remote directory.

## Prerequisites

1. Rust installed on local host
2. Remote host have port 22 opened and ssh server installed
3. Both local and remote host have rsync installed (use package manager to install or refer to [online guides](https://operavps.com/docs/install-rsync-command-in-linux/))
4. SSH keys generation and SSH config setup so that rsync client can use the correct ssh keys automatically


To generate ssh keys:
<pre>
    <code>
    ssh-keygen -t rsa -b 4096
    </code>
</pre>

For ssh config file (~/.ssh/config):
<pre>
    <code>
    Host HOSTNAME
        User USERNAME
        IdentityFile PATH_TO_IDENTITY_FILE
    </code>
</pre>

## Setup

1. Git clone this repo
2. Change variables in environment file ([.env_bak](.env_bak)) and rename it to ".env"

## Build

<pre><code>cargo build --release</code></pre>

## Development

<pre><code>cargo run</code></pre>
