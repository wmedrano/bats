(add-to-load-path "./")
(use-modules (bats)
             (system repl repl))

(define (make-track-with-any-instrument!)
  "Make a new track with any instrument.

Useful for testing."
  (let ((instrument (car (instrument-plugins))))
    (make-track! #:plugins (list (assoc-ref instrument 'id)))))

(activate-logging!)
(make-track-with-any-instrument!)
(start-repl)
