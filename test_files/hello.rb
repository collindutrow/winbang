puts "Hello from ruby"

File.open("test.txt", "w") do |file|
  file.write("Hello from ruby")
end

puts "Press Enter to exit..."
STDIN.gets