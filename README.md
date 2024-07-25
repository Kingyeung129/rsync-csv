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

## Script workflow

1. The script instantiates a watcher using notify crate to watch for file directory changes. 
   - An asynchronous channel instantiated to send and receive data from file watcher
   - Recursive mode is defined to ensure that all sub directories will also be watched.
2. Once file changes is detected, check if file event file extension is "csv". If yes match file event kind to be either Create / Modify data event.
3. Once file event matches, add to event vector and update last matched event variable to the timestamp on file event match.
4. If last matched event timestamp have elapsed over specified environment variable "CSV_EVENT_WAIT_SECONDS" or event vector length exceeds specified environment variable "CSV_EVENT_UPPER_LIMIT", proceed on with csv file processing.
5. In the processing phase, the following 5 operations will be performed:
   1. Match csv file column headers with template csv files in directory specified in environment variable "TEMPLATE_DIR"
      - Note that all csv template files name should be suffixed with "_template". The csv template file name base word should be the database table name. Example, for "anthropometry_template.csv" -> "anthropometry" will be the table name.
      - Script will read all template csv in "TEMPLATE DIR" and store them as hashmap for matching (keys for hashmap will be the column headers, while values will be the table name)
      - Currently, column headers ordering is static and must follow those defined in csv templates. If not, no match will be returned.
   2. On match, create metadata file containing timestamp of upload, user and file name
   3. Create a hashmap for rsync operations.
      - Components
        - **table_name:**

            Type: String
            
            Description: The key representing the name of the database table. Each table name is unique within the hashmap and serves as an identifier for the associated files.

        - **src_files:**

            Type: Array of Strings (`Vec<String>`)
            
            Description: A list of source file paths (csv files) associated with the table. These files contain the primary data that needs to be processed and loaded.
    
        - **metadata_files:**

            Type: Array of Strings (`Vec<String>`)
            
            Description: A list of metadata file paths associated with the table. These files contain metadata information related to the source files, such as upload timestamp, user and associated source file name.
<pre>
    <code>
Rsync hashmap structure (represented in json format):
{
    "table_name": {
        "src_files": [src_file...],
        "metadata_files": [metadata_file...]
    }
}
    </code>
</pre>
   4. Enumerate rsync hashmap table names and perform rsync push operations to remote directory for both csv file and metadata via command line
      - The command line arguments for source files and metadata are stringed together using native rust string join trait
      - The remote directory is created if not exist using --rsync-path argument. The remote directory follows the table name specified in the provided rsync hashmap. The --rsync-path  argument can be used to specify what program is to be run on the remote machine to start-up rsync (refer to rsync manual).
      - Due to limit on command line arguments, the arguments bounded by environment variable "CSV_EVENT_UPPER_LIMIT" should be kept within the bounds of ARG_MAX. By default, CSV_EVENT_UPPER_LIMIT=100 is a safe number.
      - ARG_MAX (bytes) can be found by running <code>getconf ARG_MAX</code>
      - A timeout on rsync command has been defined in case of network issues or ssh connection issues.
      - If rsync command fails, retry for a total of 3 times. The rsync command can fail due to timeout or ssh key exchange errors. After the third try, log out the error and continue.
   5. Update upload log file on status of upload
