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
            Only error output will be displayed


OPTIONS:
        --filter-content-type=<filter_context_type>...
            Include request that respond with specific types

        --filter-domain=<filter_domain>...
            Include requests for specificed domain

        --filter-path=<filter_path>
            A Regex to filter the path on

    -o, --output=<output>
            Output to a file instead of stdout

        --format <output_format>
            Instead of output [default: har]  [possible values: har, html, md, markdown]


ARGS:
    <INPUT>
            Input to be parsed.
```
