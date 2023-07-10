(add-to-load-path "./")
(use-modules (bats)
             (system repl repl))

(define (make-track-with-any-instrument!)
  "Make a new track with any instrument.

Useful for testing."
  (let* ((instruments (instrument-plugins))
         (instrument  (car instruments))
         (id          (assoc-ref instrument 'plugin-id))
         )
    (display id)
    (make-track! #:plugin-ids (list id))))

(activate-logging!)
(make-track-with-any-instrument!)
(start-repl)
