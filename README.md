# Documentation

This repo contains rust code to perform rsync push operations on file changes in a local directory to a remote directory. Supports following symbolic links to enumerate through subdirectories and files.

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

For ssh config file (~/.ssh/config), insert the following:
<pre>
    <code>
    # Change as appropriate
    Host HOSTNAME
        User USERNAME
        IdentityFile PATH_TO_IDENTITY_FILE
    </code>
</pre>

Add public key to authorized_keys file in remote host. Ensure that .ssh subdirectory is set with correct permissions in both local and remote hosts.

<pre>
    <code>
    chmod 700 ~/.ssh
    chmod 600 ~/.ssh/authorized_keys
    </code>
</pre>

**Note:**

1. Pay extra attention to permissions, if not ssh will fail especially in [strict mode](https://www.ibm.com/docs/en/was-liberty/nd?topic=system-avoiding-problems-ssh-in-collective).
2. Execute permission on directory is required for traversing directory/folder

## Setup

1. Git clone this repo
2. Change variables in environment file ([.env.bak](.env.bak)) and rename it to ".env"

## Build

<pre><code>cargo build --release</code></pre>

## Development

<pre><code>cargo run</code></pre>

## Production

Run binary after building.

<pre><code>./target/release/rsync_csv</code></pre>
