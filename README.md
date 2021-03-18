# Micro
Micro was (and still is) an app I wrote while learning the [rust programming language](https://www.rust-lang.org/).

The objective was to create a simple CLI tool which would generate a no brainer static blog-like website from Markdown files sitting in a directory.

Micro is fully functional it however lacks some polishing

## Using micro
Micro uses two folders which can be set from the command line but which are defaulting to
* **wwwroot** which will contain the source markdown files but also their generated html counterparts
* **templates** which should contain the templates used when generating the html content from the markdown files

### Metadata
Micro expects your markdown files to contain some YAML formatted metadata
```
---
layout: index
published-on: 2021-01-01T20:00:00Z
title: Blogging Like a Boss  
description: This is a long long description
  and uses multi line description
  even two lines
---
```
The following properties are supported
* **layout** : name of the template to be used when converting the file. Currently only **article** and **index** are supported
* **published-on** : publication date ISO formatted
* **title** : title of the page
* **description** : short description of the page

Meta data properties can be used in templates using the mustache syntax :

```
<body>
  <header>
    <h1>{{title}}</h1>
    <h5>{{published-on}}</h5>
  </header>

  <main>
    <article>
      {{content}}
    </article>
  </main>

  <footer>
  </footer>
</body>
```

## Running Micro
### Development/Authoring mode
Starting micro using the *--dev* switch :
```
>./micro.exe --dev
```
Will start a local webserver and will start serving your content. The server includes a websocket server which will notify clients of page changes (see uplink.js) to automatically reload the pages your are modifying

### Republishing
To republish all your pages use the publish subcommand
```
>./micro.exe publish
```
Unless using the --force option switch publish will only publish file which actually need to be regenerated (changed markdown, updated template file)