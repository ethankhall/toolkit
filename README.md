# Toolkit

Toolkit contains a set of tools useful to development.

## Time
Time is hard and my Flux Capacitor is in the shop. So the `time` command takes a bunch of different inputs, and tells you details about it. This is useful when you need to go between UTC, PST, and UNIX Epoch.

```
$> toolkit time now
Understood the date was 2018-12-06 21:08:49.270120100 -08:00

     Standard Format in UTC || Thu Dec  6 21:08:49 2018
    Standard Format with Tz || 2018-12-06T21:08:49.270120100-08:00
                 UNIX EPOCH || 1544159329
            UNIX EPOCH (ms) || 1544159329270
     Rendered Format (Orig) || Thu Dec 06 21:08:49 -08:00 2018
      Rendered Format (UTC) || Fri Dec 07 05:08:49 UTC 2018
  Rendered Format (Chicago) || Thu Dec 06 23:08:49 CST 2018
   Rendered Format (LA/SEA) || Thu Dec 06 21:08:49 PST 2018
             Year-Month-Day || 2018-12-06
             Month/Day/Year || 12/06/18
                   YYYYMMDD || 20181206
```

## Har
Har is a format you can get from Chrome/Firefox. These files are large. The `har` command allows you to filter the file by content-type, domain, and url. Once you've filtered the file, you can export in Har format for other tools, or into HTML or Markdown for easy easing.

An example usecase would look like
```
$> toolkit har --filter-domain=google.com --filter-content-type=application/json --format=html google-api.har > google-api.html
```

### Help
```
$> toolkit har --help
toolkit-har
Take a Har file, apply some filtering, then output a new Har file

USAGE:
    toolkit har [FLAGS] [OPTIONS] <INPUT>

FLAGS:
    -d, --debug
            Turn debugging information on

    -h, --help
            Prints help information

    -q, --quite
            Only error output will be displayed

    -V, --version
            Prints version information

    -w, --warn
            Warning and Error level output will be displayed


OPTIONS:
        --filter-content-type <filter_context_type>...
            Include request that respond with specific types

        --filter-domain <filter_domain>...
            Include requests for specificed domain

        --filter-path <filter_path>...
            A Regex to filter the path on

    -o, --output <output>
            Output to a file instead of stdout

    -f, --format <output_format>
            Change the output format [default: har]  [possible values: har, html, md, markdown]


ARGS:
    <INPUT>
            Path to file to process
```

## NSQ
[NSQ](https://nsq.io/) is a realtime distributed messaging platform. It's got cloud. 

Sometimes you don't want cloud scale and just want to send some data to NSQ and *NOT* blow up everything. `nsq send` will send messages to NSQ and check how backed up the tubes have gotten. It will aim to keep 1000 messages waiting to be processed. There is also some rate limiting if you want to send things slower, but the queue backup is hard coded.

### Help
```
Send a \n deliminated file to an NSQ topic

USAGE:
    toolkit nsq send [FLAGS] [OPTIONS] <TOPIC> <INPUT> --lookupd-host <nsq_lookup_host>

FLAGS:
    -d, --debug      Turn debugging information on
    -h, --help       Prints help information
    -q, --quite      Only error output will be displayed
    -V, --version    Prints version information
    -w, --warn       Warning and Error level output will be displayed

OPTIONS:
        --limit <limit>                     Limit the number of posts we send
        --lookupd-host <nsq_lookup_host>    Host to NSQ Lookup
        --lookupd-port <nsq_lookup_port>    Port to NSQ Lookup [default: 4161]
        --offset <offset>                   Where in the file to start posting
        --rate <rate>                       Limit the rate we send posts [default: 200]

ARGS:
    <TOPIC>    Which topic should be posted to
    <INPUT>    File to post line by line to the Bus
```

## JSON
Oh JSON my JSON.

`json filter` helps keep things fresh! It's useful for when you have a bunch of newline delimited json, and want the freshest one. Sometimes there are multiple messages with for the same thing, but with new-er values.

If you happen to have a list JSON blobs line by line and want to find the _most recent_ one there. This tool will do that filtering for you!

### Help
```
If a JSON blob has both an ID that's unique, and a timestamp/version field. Filter the stream for the latest ID/version
field.

USAGE:
    toolkit json filter [FLAGS] <OUTPUT> --id-path <id> --sequence-path <seq>

FLAGS:
    -d, --debug      Turn debugging information on
    -h, --help       Prints help information
    -q, --quite      Only error output will be displayed
    -V, --version    Prints version information
    -w, --warn       Warning and Error level output will be displayed

OPTIONS:
        --id-path <id>           A field like a ID or GUID that will be unique between different logical units, but the
                                 same for the same unit at different times
        --sequence-path <seq>    Path to a value that will be greater than a previous value, based on order the the blob
                                 was created

ARGS:
    <OUTPUT>    File to write output to
```