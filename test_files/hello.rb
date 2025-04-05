#!/usr/bin/env ruby
puts "Hello, world!"

# Write "Hello, world!" to a file named hello.txt
# in: C:\Users\aureate\src\localhost\rs-winshim\target\debug\hello.txt
File.open("C:\\Users\\aureate\\src\\localhost\\rs-winshim\\target\\debug\\hello.txt", "w") do |file|
  file.write("Hello, world!")
end

# Press the Enter key to exit the program
puts "Press Enter to exit..."
STDIN.gets