function Link(link)
  link.target = string.gsub(link.target, "%.md", ".html")
  return link
end
