---
title: Tutorial
index: 1
---

# Tutorial

If you are new to Docgen, this tutorial will walk you through getting your site built and
deployed.

## Installation

First, make sure that you have installed Docgen locally. Follow the instructions in the
[installation guide](/installing).

To verify you have installed everything correctly, run the following:

```
$ docgen --version
Docgen x.y.z
```

## Creating a new site

Creating a new documentation site can be done easily with the `docgen init` command:

```
$ docgen init
...
```

This will create a `docs/` directory in the root of your repository, and some pages for you to get
started with.

If you wish to use a different directory than `docs/`, you can pass the name of that directory as the argument `--docs-dir`:

```
$ docgen init --docs-dir some_subdirectory
...
```

You'll also find a `docgen.yaml` in your project root now. Lets take a look at it.

```
# On Mac / Linux
$ cat docgen.yaml
---
title: "My project"



# On Windows
$ type docgen.yaml
---
title: "My project"

```

Currently, you only have the project's name mentioned. This title is shown on the page navigation,
and used as the HTML page title. You should change that to be the actual name of your project.

Now, you can run `docgen serve` to start the local webserver. _**Note**: When you edit your
`docatave.yaml` file, you will have to restart the webserver for the changes to come into effect._

```
$ docgen serve

Docgen | Serve
Starting development server...

Server running on http://0.0.0.0:4001/
```

And finally, go to [http://localhost:4001](http://localhost:4001) to view your site.

## Editing content

While `docgen serve` is running, you can edit the your documentation Markdown files, and you will
immediately see your page update. Try it! Open up `docs/README.md` in your favorite text editor,
and make a change. You should see the browser refresh and show your changes automatically. This way
you can quickly see what your changes look like.

If you are not familiar with Markdown syntax, or need a refresher, you can read our [Markdown
reference](/features/markdown) or check out [this
guide](https://guides.github.com/features/mastering-markdown/) by GitHub. Note that there are a few
different flavors of Markdown. Docgen supports all the "basics" Markdown features, as well as a few
"GitHub flavored Markdown" features - namely task lists and tables.

## Adding pages

To add a new page, all you need to do is add another Markdown file.

Let's say you want to add another page; a "How To Build" page. All you need to do is create a that
page inside your docs folder.

```
# On Mac / Linux
$ touch docs/building.md


# On Windows
$ echo.> docs\building.md
```

By default, Docgen assumes the title of the page is its title, but we may want to change that.
Let's add a _front matter block_ to the page. This is just a quick
[YAML](https://blog.stackpath.com/yaml/) snippet that gives Docgen some additional information
about the page.

Paste the following into the file you just created:

```
---
title: How to build
---

# How to build

...
```

Open up your browser, and you should now see a _"How to build"_ link in the sidebar. And if you
click it, you will be taken to the page.

## What next?

That's really all there is to know about getting started with Docgen. Here are some resources that
can help going forward:

- [Deployment guide](/deployment)
- [Adding Mermaid.js diagrams](/features/mermaid-js)
- [Customize your look and feel](/features/look-and-feel)
- [Customize your navigation](/features/custom-navigation)
